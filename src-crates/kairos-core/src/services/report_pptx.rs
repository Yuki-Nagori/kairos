//! 报告 PPTX 导出：把报告章节写成演示文稿。
//!
//! 生成放在 core（离线、无 UI 依赖），命令层只负责落盘与打开。用 `ppt-rs`
//! （Apache-2.0）而不是自写 OOXML：幻灯片 XML 的骨架（关系、内容类型、主题）
//! 属于「通用问题」，自己写只是把同一份样板再抄一遍。

use crate::error::{KairosError, Result};

/// 一页幻灯片：标题 + 要点。
pub struct ReportSlide {
    pub title: String,
    pub bullets: Vec<String>,
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

    #[test]
    fn empty_report_is_rejected() {
        let error = build_report_deck("空报告", &[]).unwrap_err();
        assert_eq!(error.kind(), crate::error::ErrorKind::Validation);
    }
}
