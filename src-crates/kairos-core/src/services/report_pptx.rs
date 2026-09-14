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

/// 从 dataURL 还原快照：解 base64、按魔数读尺寸、认扩展名。
///
/// 尺寸不信前端给的数字，直接从字节读（PNG 的 IHDR 是权威且必然存在）——
/// 前端只说「这是哪张图」，排版参数由导出侧决定。
pub fn image_from_data_url(data_url: &str) -> Result<ReportImage> {
    let (header, payload) = data_url
        .split_once(',')
        .ok_or_else(|| KairosError::validation("快照不是 dataURL 形态（缺少逗号分隔的头部）。"))?;
    let format = if header.contains("image/png") {
        "PNG"
    } else if header.contains("image/jpeg") {
        "JPEG"
    } else {
        return Err(KairosError::validation(
            "仅支持 PNG / JPEG 快照，其它格式请先在界面上改为导出图片。",
        ));
    };
    if !header.contains("base64") {
        return Err(KairosError::validation("快照编码不是 base64。"));
    }
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, payload)
        .map_err(|e| KairosError::validation(format!("快照解码失败：{e}")))?;
    let (width_px, height_px) = match format {
        "PNG" => png_dimensions(&bytes),
        _ => jpeg_dimensions(&bytes),
    }
    .ok_or_else(|| KairosError::validation(format!("快照尺寸无法读取（{format} 头部不完整）。")))?;
    Ok(ReportImage {
        bytes,
        width_px,
        height_px,
        format: format.to_string(),
    })
}

/// PNG 尺寸：IHDR 在固定偏移（签名 8 字节 + 长度 4 + 类型 4 之后，宽高各 4 字节大端）。
fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() < 24 || bytes[..8] != PNG_SIGNATURE || &bytes[12..16] != b"IHDR" {
        return None;
    }
    // 长度已在上方校验，这里按索引直取：不用 `?` 转换，避免覆盖率把错误落点算成未执行
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    (width > 0 && height > 0).then_some((width, height))
}

/// JPEG 尺寸：扫描 SOFn 段（FFC0–FFCF，跳过 FF C4 / C8 / CC 这几个非 SOF 标记）。
fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return None;
    }
    let mut offset = 2usize;
    while offset + 9 < bytes.len() {
        if bytes[offset] != 0xFF {
            offset += 1;
            continue;
        }
        let marker = bytes[offset + 1];
        let is_sof = (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC);
        if is_sof {
            let height = u16::from_be_bytes([bytes[offset + 5], bytes[offset + 6]]);
            let width = u16::from_be_bytes([bytes[offset + 7], bytes[offset + 8]]);
            return (width > 0 && height > 0).then_some((width as u32, height as u32));
        }
        let length = u16::from_be_bytes([bytes[offset + 2], bytes[offset + 3]]) as usize;
        if length < 2 {
            return None;
        }
        offset += 2 + length;
    }
    None
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
    fn data_url_decodes_png_and_reads_its_size() {
        use base64::Engine as _;
        let payload = base64::engine::general_purpose::STANDARD.encode(PIXEL_PNG);
        let image = image_from_data_url(&format!("data:image/png;base64,{payload}")).unwrap();
        assert_eq!((image.width_px, image.height_px), (1, 1));
        assert_eq!(image.format, "PNG");
        assert_eq!(image.bytes, PIXEL_PNG);
    }

    #[test]
    fn data_url_decodes_jpeg_and_reads_its_size() {
        // 走公共入口（不只测内部的 JPEG 扫描器）：JPEG 快照同样能进 deck
        use base64::Engine as _;
        let jpeg: [u8; 21] = [
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00,
            0x02, 0x00, 0x03, 0x01, 0x00, 0x11, 0x00,
        ];
        let payload = base64::engine::general_purpose::STANDARD.encode(jpeg);
        let image = image_from_data_url(&format!("data:image/jpeg;base64,{payload}")).unwrap();
        assert_eq!(image.format, "JPEG");
        assert_eq!((image.width_px, image.height_px), (3, 2));
        // PNG 头部不完整 → 报错而不是当作 0×0
        assert!(png_dimensions(&PIXEL_PNG[..10]).is_none());
        assert!(png_dimensions(&[0u8; 32]).is_none());
    }

    #[test]
    fn data_url_rejects_unsupported_or_malformed_input() {
        // 非 dataURL 形态
        assert!(image_from_data_url("data:image/png;base64").is_err());
        // 不支持的图片格式
        assert!(image_from_data_url("data:image/webp;base64,AAAA").is_err());
        // 声明 base64 却不是
        assert!(image_from_data_url("data:image/png,AAAA").is_err());
        // base64 内容非法
        assert!(image_from_data_url("data:image/png;base64,不是base64").is_err());
        // 头部像 PNG 但字节不是（尺寸读不出来）
        assert!(image_from_data_url("data:image/png;base64,QUJD").is_err());
    }

    #[test]
    fn jpeg_dimensions_reads_sof_and_skips_other_markers() {
        // 最小 JPEG 头部：SOI + 一个非 SOF 段（FF E0，长度 4）+ SOF0（高 2、宽 3）
        let jpeg = [
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00,
            0x02, 0x00, 0x03, 0x01, 0x00, 0x11, 0x00,
        ];
        assert_eq!(jpeg_dimensions(&jpeg), Some((3, 2)));
        // 非标记字节（高位不足 0xFF）会被跳过：在 SOI 与 SOF 之间塞一个 0x00
        let mut with_filler = jpeg.to_vec();
        with_filler.insert(2, 0x00);
        assert_eq!(jpeg_dimensions(&with_filler), Some((3, 2)));
        // 长度字段为 0 → 无法推进，返回 None 而不是死循环
        let broken = [
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert_eq!(jpeg_dimensions(&broken), None);
        // 非 JPEG
        assert_eq!(jpeg_dimensions(&[0x00, 0x01, 0x02]), None);
        // 走完全部标记都没有 SOF（例如只有 APP0）→ 返回 None
        let no_sof = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00];
        assert_eq!(jpeg_dimensions(&no_sof), None);
    }

    #[test]
    fn empty_report_is_rejected() {
        let error = build_report_deck("空报告", &[]).unwrap_err();
        assert_eq!(error.kind(), crate::error::ErrorKind::Validation);
    }
}
