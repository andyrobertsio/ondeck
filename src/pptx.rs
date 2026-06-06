//! PPTX export (image-per-slide). Each slide is captured to a PNG via headless
//! Chrome (`?shot=N`), then packed into a minimal, valid .pptx where every slide
//! is a single full-bleed picture. Pixel-identical to the HTML/PDF; not editable.

use std::io::Write;
use std::path::Path;
use std::process::Command;

use zip::write::SimpleFileOptions;

// 16:9 slide in EMU (1 inch = 914400 EMU; 13.333" × 7.5").
const SLIDE_CX: i64 = 12192000;
const SLIDE_CY: i64 = 6858000;

/// Capture slide `n` (1-based) of `html` to `png_path` via headless Chrome.
fn shoot(browser: &str, html_url: &str, n: usize, png_path: &Path) -> Result<(), String> {
    let status = Command::new(browser)
        .arg("--headless=new")
        .arg("--disable-gpu")
        .arg("--hide-scrollbars")
        .arg("--force-color-profile=srgb")
        .arg("--window-size=1920,1080")
        .arg("--virtual-time-budget=1500")
        .arg(format!("--screenshot={}", png_path.display()))
        .arg(format!("{html_url}?shot={n}"))
        .status()
        .map_err(|e| format!("running {browser}: {e}"))?;
    if !status.success() {
        return Err(format!(
            "{browser} exited with {status} capturing slide {n}"
        ));
    }
    if !png_path.exists() {
        return Err(format!("no screenshot produced for slide {n}"));
    }
    Ok(())
}

pub fn export(html_path: &Path, out_pptx: &Path, slides: usize) -> Result<(), String> {
    if slides == 0 {
        return Err("nothing to export (0 slides)".to_string());
    }
    let browser = crate::pdf::find_browser().ok_or_else(|| {
        "no Chrome/Chromium/Edge/Brave found — set DECK_CHROME=/path/to/browser".to_string()
    })?;
    let abs = std::fs::canonicalize(html_path)
        .map_err(|e| format!("resolving {}: {e}", html_path.display()))?;
    let url = format!("file://{}", abs.display());

    // Capture each slide to a PNG in a temp dir.
    let tmp = std::env::temp_dir().join(format!("deck-pptx-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| format!("temp dir: {e}"))?;
    let mut pngs = Vec::with_capacity(slides);
    for n in 1..=slides {
        let p = tmp.join(format!("slide{n}.png"));
        eprintln!("  capturing slide {n}/{slides}…");
        shoot(&browser, &url, n, &p)?;
        let bytes = std::fs::read(&p).map_err(|e| format!("reading {}: {e}", p.display()))?;
        pngs.push(bytes);
    }

    write_pptx(out_pptx, &pngs).map_err(|e| format!("writing {}: {e}", out_pptx.display()))?;
    let _ = std::fs::remove_dir_all(&tmp);
    Ok(())
}

fn write_pptx(out: &Path, pngs: &[Vec<u8>]) -> Result<(), String> {
    let file = std::fs::File::create(out).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let xml = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let raw = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let put = |zip: &mut zip::ZipWriter<std::fs::File>,
               name: &str,
               data: &[u8],
               opts|
     -> Result<(), String> {
        zip.start_file(name, opts).map_err(|e| e.to_string())?;
        zip.write_all(data).map_err(|e| e.to_string())?;
        Ok(())
    };

    let n = pngs.len();
    put(
        &mut zip,
        "[Content_Types].xml",
        content_types(n).as_bytes(),
        xml,
    )?;
    put(&mut zip, "_rels/.rels", ROOT_RELS.as_bytes(), xml)?;
    put(
        &mut zip,
        "ppt/presentation.xml",
        presentation(n).as_bytes(),
        xml,
    )?;
    put(
        &mut zip,
        "ppt/_rels/presentation.xml.rels",
        presentation_rels(n).as_bytes(),
        xml,
    )?;
    put(&mut zip, "ppt/theme/theme1.xml", THEME.as_bytes(), xml)?;
    put(
        &mut zip,
        "ppt/slideMasters/slideMaster1.xml",
        SLIDE_MASTER.as_bytes(),
        xml,
    )?;
    put(
        &mut zip,
        "ppt/slideMasters/_rels/slideMaster1.xml.rels",
        MASTER_RELS.as_bytes(),
        xml,
    )?;
    put(
        &mut zip,
        "ppt/slideLayouts/slideLayout1.xml",
        SLIDE_LAYOUT.as_bytes(),
        xml,
    )?;
    put(
        &mut zip,
        "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
        LAYOUT_RELS.as_bytes(),
        xml,
    )?;

    for (i, png) in pngs.iter().enumerate() {
        let k = i + 1;
        put(&mut zip, &format!("ppt/media/image{k}.png"), png, raw)?;
        put(
            &mut zip,
            &format!("ppt/slides/slide{k}.xml"),
            slide_xml().as_bytes(),
            xml,
        )?;
        put(
            &mut zip,
            &format!("ppt/slides/_rels/slide{k}.xml.rels"),
            slide_rels(k).as_bytes(),
            xml,
        )?;
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

fn content_types(n: usize) -> String {
    let mut s = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Default Extension="png" ContentType="image/png"/>
<Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
<Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
<Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
<Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
"#,
    );
    for k in 1..=n {
        s.push_str(&format!(
            "<Override PartName=\"/ppt/slides/slide{k}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>\n"
        ));
    }
    s.push_str("</Types>");
    s
}

const ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#;

fn presentation(n: usize) -> String {
    let mut ids = String::new();
    for k in 1..=n {
        ids.push_str(&format!("<p:sldId id=\"{}\" r:id=\"rId{k}\"/>", 255 + k));
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rIdMaster"/></p:sldMasterIdLst>
<p:sldIdLst>{ids}</p:sldIdLst>
<p:sldSz cx="{SLIDE_CX}" cy="{SLIDE_CY}"/>
<p:notesSz cx="6858000" cy="9144000"/>
</p:presentation>"#
    )
}

fn presentation_rels(n: usize) -> String {
    let mut s = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rIdMaster" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
<Relationship Id="rIdTheme" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
"#,
    );
    for k in 1..=n {
        s.push_str(&format!(
            "<Relationship Id=\"rId{k}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide{k}.xml\"/>\n"
        ));
    }
    s.push_str("</Relationships>");
    s
}

fn slide_xml() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
<p:pic>
<p:nvPicPr><p:cNvPr id="2" name="Slide"/><p:cNvPicPr><a:picLocks noChangeAspect="1"/></p:cNvPicPr><p:nvPr/></p:nvPicPr>
<p:blipFill><a:blip r:embed="rIdImg"/><a:stretch><a:fillRect/></a:stretch></p:blipFill>
<p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{SLIDE_CX}" cy="{SLIDE_CY}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr>
</p:pic>
</p:spTree></p:cSld>
<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sld>"#
    )
}

fn slide_rels(_k: usize) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rIdImg" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image{_k}.png"/>
<Relationship Id="rIdLayout" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#
    )
}

const SLIDE_LAYOUT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="blank" preserve="1">
<p:cSld name="Blank"><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
</p:spTree></p:cSld>
<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sldLayout>"#;

const LAYOUT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
</Relationships>"#;

const SLIDE_MASTER: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
</p:spTree></p:cSld>
<p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
<p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst>
</p:sldMaster>"#;

const MASTER_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/>
</Relationships>"#;

const THEME: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="deck">
<a:themeElements>
<a:clrScheme name="deck">
<a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
<a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
<a:dk2><a:srgbClr val="1F1B16"/></a:dk2>
<a:lt2><a:srgbClr val="EEEEEE"/></a:lt2>
<a:accent1><a:srgbClr val="7AA2F7"/></a:accent1>
<a:accent2><a:srgbClr val="BB9AF7"/></a:accent2>
<a:accent3><a:srgbClr val="D3FB52"/></a:accent3>
<a:accent4><a:srgbClr val="FF5D8F"/></a:accent4>
<a:accent5><a:srgbClr val="B4341C"/></a:accent5>
<a:accent6><a:srgbClr val="1C6B5A"/></a:accent6>
<a:hlink><a:srgbClr val="7AA2F7"/></a:hlink>
<a:folHlink><a:srgbClr val="BB9AF7"/></a:folHlink>
</a:clrScheme>
<a:fontScheme name="deck">
<a:majorFont><a:latin typeface="Helvetica"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont>
<a:minorFont><a:latin typeface="Helvetica"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont>
</a:fontScheme>
<a:fmtScheme name="deck">
<a:fillStyleLst>
<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
</a:fillStyleLst>
<a:lnStyleLst>
<a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
<a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
<a:ln w="19050"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
</a:lnStyleLst>
<a:effectStyleLst>
<a:effectStyle><a:effectLst/></a:effectStyle>
<a:effectStyle><a:effectLst/></a:effectStyle>
<a:effectStyle><a:effectLst/></a:effectStyle>
</a:effectStyleLst>
<a:bgFillStyleLst>
<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
</a:bgFillStyleLst>
</a:fmtScheme>
</a:themeElements>
</a:theme>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ooxml_scales_with_slide_count() {
        // ".slide+xml" matches the slide content type only (not slideMaster/Layout).
        assert_eq!(content_types(3).matches(".slide+xml").count(), 3);
        assert_eq!(presentation(3).matches("<p:sldId ").count(), 3);
        assert_eq!(
            presentation_rels(3)
                .matches("/relationships/slide\"")
                .count(),
            3
        );
        assert!(presentation(1).contains(&format!("cx=\"{SLIDE_CX}\"")));
    }
}
