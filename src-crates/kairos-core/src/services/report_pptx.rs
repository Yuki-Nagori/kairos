//! 报告 PPTX 导出：把报告章节写成演示文稿。
//!
//! 生成放在 core（离线、无 UI 依赖），命令层只负责落盘与打开。用 `ppt-rs`
//! （Apache-2.0）而不是自写 OOXML：幻灯片 XML 的骨架（关系、内容类型、主题）
//! 属于「通用问题」，自己写只是把同一份样板再抄一遍。

use crate::error::{KairosError, Result};

/// 幻灯片里的位图（快照）：原始字节 + 像素尺寸 + 格式（PNG / JPEG）。
///
/// 前端给的是 dataURL，解码成字节由命令层做（core 不碰 base64 与网络）。
pub struct ReportImage {
    pub bytes: Vec<u8>,
    pub width_px: u32,
    pub height_px: u32,
    pub format: String,
}

/// 一页幻灯片：标题 + 要点 +（可选）快照。
pub struct ReportSlide {
    pub title: String,
    pub bullets: Vec<String>,
    pub images: Vec<ReportImage>,
}

/// 幻灯片可用区（EMU）：10 × 7.5 英寸版面留出标题与边距后的插图区。
const SLIDE_WIDTH_EMU: u64 = 10 * 914_400;
const MAX_IMAGE_WIDTH_EMU: u64 = 7 * 914_400;
const MAX_IMAGE_HEIGHT_EMU: u64 = 4 * 914_400;
/// 1 像素 = 9525 EMU（96 DPI，库的换算口径）。
const EMU_PER_PIXEL: u64 = 9_525;

/// 按可用区等比缩放并居中：快照常见 800×600 以上，原尺寸会溢出幻灯片。
fn place_image(image: ReportImage) -> ppt_rs::Image {
    let width_emu = (image.width_px as u64).saturating_mul(EMU_PER_PIXEL).max(1);
    let height_emu = (image.height_px as u64)
        .saturating_mul(EMU_PER_PIXEL)
        .max(1);
    let scale = (MAX_IMAGE_WIDTH_EMU as f64 / width_emu as f64)
        .min(MAX_IMAGE_HEIGHT_EMU as f64 / height_emu as f64)
        .min(1.0);
    let width_px = ((image.width_px as f64) * scale).round().max(1.0) as u32;
    let height_px = ((image.height_px as f64) * scale).round().max(1.0) as u32;
    let x = (SLIDE_WIDTH_EMU.saturating_sub(width_px as u64 * EMU_PER_PIXEL)) / 2;
    ppt_rs::Image::from_bytes(image.bytes, width_px, height_px, &image.format)
        .position(x as u32, 1_200_000)
}

/// 生成演示文稿字节流（调用方负责写盘）。
///
/// 空要点列表允许：报告里有些章节只有标题与一句结论。
pub fn build_report_deck(title: &str, slides: &[ReportSlide]) -> Result<Vec<u8>> {
    if slides.is_empty() {
        return Err(KairosError::validation("报告没有可导出的章节。"));
    }
    let contents: Vec<ppt_rs::SlideContent> = slides
        .iter()
        .map(|slide| {
            let mut content = ppt_rs::SlideContent::new(&slide.title);
            for bullet in &slide.bullets {
                content = content.add_bullet(bullet);
            }
            if !slide.images.is_empty() {
                content.has_image = true;
                content = content.with_images(
                    slide
                        .images
                        .iter()
                        .map(|image| {
                            place_image(ReportImage {
                                bytes: image.bytes.clone(),
                                width_px: image.width_px,
                                height_px: image.height_px,
                                format: image.format.clone(),
                            })
                        })
                        .collect(),
                );
            }
            content
        })
        .collect();
    ppt_rs::create_pptx_with_content(title, contents).map_err(deck_error)
}

/// 库错误 → 领域错误。抽成命名函数而不是内联闭包：闭包只会在库报错时执行，
/// 那条路径在测试里不可达，行覆盖会把它记成「未覆盖」；命名函数可以直接被测。
fn deck_error(error: impl std::fmt::Debug) -> KairosError {
    KairosError::internal(format!("生成演示文稿失败：{error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn read_part(bytes: &[u8], name: &str) -> String {
        let reader = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader).expect("产物应是合法 zip");
        let mut file = archive.by_name(name).expect("缺少预期部件");
        let mut text = String::new();
        file.read_to_string(&mut text).expect("部件应是 UTF-8");
        text
    }

    #[test]
    fn deck_is_a_zip_with_slide_parts() {
        let deck = build_report_deck(
            "控制器支架 · 分析报告",
            &[ReportSlide {
                title: "填充分析".to_string(),
                bullets: vec!["V/P 切换 1.08 s".to_string(), "保压 59.3 MPa".to_string()],
                images: Vec::new(),
            }],
        )
        .expect("生成应成功");
        // ZIP 魔数：Office 文档就是 zip 包
        assert_eq!(&deck[..2], b"PK");
        let slide = read_part(&deck, "ppt/slides/slide1.xml");
        // 中文保真：标题与要点原样出现在幻灯片 XML 里（不是转义或乱码）
        assert!(slide.contains("填充分析"), "{slide}");
        assert!(slide.contains("V/P 切换 1.08 s"), "{slide}");
        let presentation = read_part(&deck, "ppt/presentation.xml");
        assert!(!presentation.is_empty());
    }

    #[test]
    fn library_error_maps_to_internal() {
        let error = deck_error("boom");
        assert_eq!(error.kind(), crate::error::ErrorKind::Internal);
        assert!(error.message().contains("boom"));
    }

    /// 1×1 像素 PNG（最小合法图片，用于验证插图路径）。
    const PIXEL_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn slide_with_snapshot_embeds_an_image_part() {
        let deck = build_report_deck(
            "报告",
            &[ReportSlide {
                title: "视口快照".to_string(),
                bullets: vec!["T 场".to_string()],
                images: vec![ReportImage {
                    bytes: PIXEL_PNG.to_vec(),
                    width_px: 1200,
                    height_px: 800,
                    format: "PNG".to_string(),
                }],
            }],
        )
        .expect("生成应成功");
        // 包里出现媒体部件，且该页引用了它（r:embed 关系）
        let reader = std::io::Cursor::new(&deck);
        let mut archive = zip::ZipArchive::new(reader).expect("合法 zip");
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(
            names.iter().any(|name| name.starts_with("ppt/media/")),
            "{names:?}"
        );
        let slide = read_part(&deck, "ppt/slides/slide1.xml");
        assert!(slide.contains("r:embed"), "{slide}");
        // 等比缩放：1200×800 的原始尺寸是 11,430,000 × 7,620,000 EMU，会溢出 10×7.5 英寸
        // 版面；缩放后不应再出现原始宽度（具体 EMU 由库的换算与取整决定，不锁死数字）
        assert!(
            !slide.contains("11430000"),
            "插图未按可用区缩放（仍是原始尺寸）"
        );
    }

    #[test]
    fn empty_report_is_rejected() {
        let error = build_report_deck("空报告", &[]).unwrap_err();
        assert_eq!(error.kind(), crate::error::ErrorKind::Validation);
    }
}
