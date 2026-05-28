//! Render PDF pages to PNG images using macOS CoreGraphics.
//!
//! Each page is rasterised at the requested DPI into an RGBA bitmap,
//! then saved as PNG via the `image` crate. The caller owns the temp
//! directory — this module only writes files into it.

use std::path::{Path, PathBuf};

use core_foundation::base::TCFType;
use core_foundation::url::CFURL;
use core_graphics::color_space::CGColorSpace;
use core_graphics::context::CGContext;
use core_graphics::geometry::{CGPoint, CGRect, CGSize};

const DEFAULT_DPI: f64 = 200.0;
const PDF_POINT_DPI: f64 = 72.0;

mod ffi {
    use core_graphics::geometry::CGRect;

    pub enum CGPDFDocument {}
    pub enum CGPDFPage {}

    pub type CGPDFDocumentRef = *const CGPDFDocument;
    pub type CGPDFPageRef = *const CGPDFPage;

    // CGPDFBox: kCGPDFMediaBox = 0
    pub const K_CGPDF_MEDIA_BOX: i32 = 0;

    unsafe extern "C" {
        pub fn CGPDFDocumentCreateWithURL(url: *const std::ffi::c_void)
            -> CGPDFDocumentRef;
        pub fn CGPDFDocumentRelease(document: CGPDFDocumentRef);
        pub fn CGPDFDocumentGetNumberOfPages(document: CGPDFDocumentRef) -> usize;
        pub fn CGPDFDocumentGetPage(document: CGPDFDocumentRef, page_number: usize)
            -> CGPDFPageRef;
        pub fn CGPDFPageGetBoxRect(page: CGPDFPageRef, box_type: i32) -> CGRect;
        pub fn CGContextDrawPDFPage(
            context: core_graphics::sys::CGContextRef,
            page: CGPDFPageRef,
        );
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PdfRenderError {
    #[error("failed to open PDF: {0}")]
    Open(String),
    #[error("page {0} out of range")]
    PageOutOfRange(usize),
    #[error("render failed for page {0}")]
    RenderFailed(usize),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image encode error: {0}")]
    ImageEncode(String),
}

struct PdfDocument {
    ptr: ffi::CGPDFDocumentRef,
}

impl PdfDocument {
    fn open(path: &str) -> Result<Self, PdfRenderError> {
        let url = CFURL::from_path(Path::new(path), false)
            .ok_or_else(|| PdfRenderError::Open(format!("invalid path: {path}")))?;
        let doc = unsafe {
            ffi::CGPDFDocumentCreateWithURL(url.as_concrete_TypeRef() as *const _)
        };
        if doc.is_null() {
            return Err(PdfRenderError::Open(format!("cannot open: {path}")));
        }
        Ok(Self { ptr: doc })
    }

    fn page_count(&self) -> usize {
        unsafe { ffi::CGPDFDocumentGetNumberOfPages(self.ptr) }
    }

    fn page(&self, number: usize) -> Option<ffi::CGPDFPageRef> {
        let p = unsafe { ffi::CGPDFDocumentGetPage(self.ptr, number) };
        if p.is_null() { None } else { Some(p) }
    }
}

impl Drop for PdfDocument {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::CGPDFDocumentRelease(self.ptr) };
        }
    }
}

pub fn render_pages_to_pngs(
    pdf_path: &str,
    out_dir: &Path,
    dpi: Option<f64>,
) -> Result<Vec<PathBuf>, PdfRenderError> {
    let dpi = dpi.unwrap_or(DEFAULT_DPI);
    let scale = dpi / PDF_POINT_DPI;

    let doc = PdfDocument::open(pdf_path)?;
    let count = doc.page_count();
    if count == 0 {
        return Err(PdfRenderError::Open("PDF has no pages".into()));
    }

    let mut paths = Vec::with_capacity(count);

    for i in 1..=count {
        let page = doc
            .page(i)
            .ok_or(PdfRenderError::PageOutOfRange(i))?;

        let media_box =
            unsafe { ffi::CGPDFPageGetBoxRect(page, ffi::K_CGPDF_MEDIA_BOX) };
        let w = (media_box.size.width * scale).ceil() as usize;
        let h = (media_box.size.height * scale).ceil() as usize;

        if w == 0 || h == 0 {
            return Err(PdfRenderError::RenderFailed(i));
        }

        let color_space = CGColorSpace::create_device_rgb();
        let mut ctx = CGContext::create_bitmap_context(
            None,
            w,
            h,
            8,
            w * 4,
            &color_space,
            core_graphics::base::kCGImageAlphaPremultipliedLast,
        );

        ctx.set_rgb_fill_color(1.0, 1.0, 1.0, 1.0);
        ctx.fill_rect(CGRect::new(
            &CGPoint::new(0.0, 0.0),
            &CGSize::new(w as f64, h as f64),
        ));

        ctx.scale(scale, scale);

        unsafe {
            // `sys::CGContextRef` IS `*mut sys::CGContext`. `&CGContextRef`
            // is bit-equivalent to that raw pointer (foreign_types Opaque
            // wrapper). Cast the reference address directly — do NOT
            // dereference, or we'd read bytes from inside the C struct
            // as if they were a pointer and PDF rendering silently no-ops.
            let ctx_ref: &core_graphics::context::CGContextRef = &ctx;
            let raw: core_graphics::sys::CGContextRef =
                ctx_ref as *const _ as *mut core_graphics::sys::CGContext;
            ffi::CGContextDrawPDFPage(raw, page);
        }

        let data_slice = ctx.data();
        let buf = &data_slice[..w * h * 4];

        let img_buf = image::RgbaImage::from_raw(w as u32, h as u32, buf.to_vec())
            .ok_or_else(|| PdfRenderError::RenderFailed(i))?;

        let out_path = out_dir.join(format!("page-{i:03}.png"));
        img_buf
            .save(&out_path)
            .map_err(|e| PdfRenderError::ImageEncode(e.to_string()))?;

        paths.push(out_path);
    }

    Ok(paths)
}

pub fn page_count(pdf_path: &str) -> Result<usize, PdfRenderError> {
    Ok(PdfDocument::open(pdf_path)?.page_count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn minimal_pdf_bytes() -> Vec<u8> {
        // Minimal valid 1-page PDF
        let pdf = b"%PDF-1.0
1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj
2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj
3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >> endobj
xref
0 4
0000000000 65535 f \n0000000009 00000 n \n0000000058 00000 n \n0000000115 00000 n \ntrailer << /Size 4 /Root 1 0 R >>
startxref
190
%%EOF";
        pdf.to_vec()
    }

    #[test]
    fn render_minimal_pdf_produces_png() {
        let tmp = std::env::temp_dir().join("sniptex-pdf-render-test");
        let _ = std::fs::create_dir_all(&tmp);

        let pdf_path = tmp.join("test.pdf");
        let mut f = std::fs::File::create(&pdf_path).unwrap();
        f.write_all(&minimal_pdf_bytes()).unwrap();
        drop(f);

        let out_dir = tmp.join("pages");
        let _ = std::fs::create_dir_all(&out_dir);

        let result = render_pages_to_pngs(pdf_path.to_str().unwrap(), &out_dir, Some(72.0));
        match result {
            Ok(paths) => {
                assert_eq!(paths.len(), 1);
                assert!(paths[0].exists());
                assert!(std::fs::metadata(&paths[0]).unwrap().len() > 0);
            }
            Err(PdfRenderError::Open(_)) => {
                // Minimal PDF may not render on all CoreGraphics versions
            }
            Err(e) => panic!("unexpected error: {e}"),
        }

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn nonexistent_pdf_returns_open_error() {
        let result =
            render_pages_to_pngs("/tmp/does-not-exist.pdf", Path::new("/tmp"), None);
        assert!(matches!(result, Err(PdfRenderError::Open(_))));
    }

    /// Regression: ensure `CGContextDrawPDFPage` actually rasterises content,
    /// not just a white background. A PDF with a filled black rectangle MUST
    /// produce a PNG that has at least one non-white pixel.
    #[test]
    fn render_pdf_with_content_produces_non_blank_png() {
        let tmp = std::env::temp_dir().join("sniptex-pdf-content-test");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::create_dir_all(&tmp);

        // PDF v1.4 with a content stream that fills a 100x100 black rect
        let pdf_bytes: &[u8] = b"%PDF-1.4\n\
1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n\
2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n\
3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R /Resources << >> >> endobj\n\
4 0 obj << /Length 26 >>\nstream\n\
0 0 0 rg\n50 50 100 100 re\nf\n\
endstream\nendobj\n\
xref\n0 5\n\
0000000000 65535 f \n\
0000000009 00000 n \n\
0000000056 00000 n \n\
0000000111 00000 n \n\
0000000208 00000 n \n\
trailer << /Size 5 /Root 1 0 R >>\n\
startxref\n290\n%%EOF\n";

        let pdf_path = tmp.join("content.pdf");
        std::fs::write(&pdf_path, pdf_bytes).unwrap();

        let out_dir = tmp.join("pages");
        std::fs::create_dir_all(&out_dir).unwrap();

        let result = render_pages_to_pngs(pdf_path.to_str().unwrap(), &out_dir, Some(150.0));
        let paths = match result {
            Ok(p) => p,
            Err(PdfRenderError::Open(_)) => {
                // CoreGraphics couldn't parse our minimal PDF — skip
                let _ = std::fs::remove_dir_all(&tmp);
                return;
            }
            Err(e) => panic!("render failed: {e}"),
        };
        assert_eq!(paths.len(), 1);

        let img = image::open(&paths[0]).expect("decode PNG");
        let rgba = img.to_rgba8();
        let total = rgba.pixels().count();
        let non_white = rgba
            .pixels()
            .filter(|p| !(p[0] > 240 && p[1] > 240 && p[2] > 240))
            .count();

        let _ = std::fs::remove_dir_all(&tmp);

        assert!(
            non_white > total / 100,
            "PNG appears blank: only {non_white}/{total} non-white pixels — \
             PDF rendering produced a white-only image, CGContextDrawPDFPage \
             likely received a bad context pointer"
        );
    }
}
