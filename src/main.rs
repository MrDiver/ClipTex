use arboard::{Clipboard, ImageData};
use cairo::ImageSurface;
use clipboard_master::{CallbackResult, ClipboardHandler};
use env_logger;
use log::{debug, error};
use notify_rust::Notification;
use std::{borrow::Cow, fs};
use tectonic;

fn has_latex_macro(input: &str) -> bool {
    input.starts_with("!tex ")
}

fn snip_latex(input: &str) -> String {
    input.replace("!tex ", "")
}

fn string_in_template(input: &str) -> String {
    let doc = fs::read_to_string("templates/basic.tex").expect("Failed to read tex template");
    let doc = doc.replace("%INSERT%", input);
    debug!("{}", doc);
    doc
}

fn tectonic_rendering(latex_code: &str) -> Option<(Vec<u8>, i32, i32)> {
    /* Render latex to PDF */
    let scale = 5.0;
    let pdf_result = tectonic::latex_to_pdf(latex_code);
    if let Err(error) = pdf_result {
        error.dump_uncolorized();
        error!("Failed: {}", error.description());
        return None;
    }
    let mut pdf_vec = pdf_result.unwrap();
    let bytes = pdf_vec.as_mut_slice();

    // let mut file = File::create("compare.pdf").expect("Couldn't create file");
    // file.write_all(bytes).expect("Couldn't write to pdf file");

    /* Parse PDF data with Poppler */
    let document_result = poppler::PopplerDocument::new_from_data(bytes, "");
    if let Err(error) = document_result {
        error!(
            "Something went wrong with poppler reading the bytes:\n{}",
            error
        );
        return None;
    }
    let document = document_result.unwrap();
    let page_result = document.get_page(0);
    if page_result.is_none() {
        error!("Something went wrong with reading the first page of the pdf");
        return None;
    }
    let page = page_result.unwrap();
    let (width, height) = page.get_size();
    let width = (scale * width) as i32;
    let height = (scale * height) as i32;
    // debug!("Width: {}, Height: {}", width, height);

    /* Create Cairo Context for rendering and Render PDF */
    let surface_result = ImageSurface::create(cairo::Format::ARgb32, width, height);
    if let Err(error) = surface_result {
        error!(
            "Something went wrong with creating a cairo surface\n{}",
            error
        );
        return None;
    }
    let mut surface = surface_result.unwrap();
    /* Block needed to destroy context before collection data from surface */
    {
        let cr_result = cairo::Context::new(&surface);
        if let Err(error) = cr_result {
            error!(
                "Something went wrong with creating a cairo context\n{}",
                error
            );
            return None;
        }
        let cr = cr_result.unwrap();
        cr.set_source_rgb(1.0, 1.0, 1.0);
        if let Err(error) = cr.paint() {
            error!("Failed to paint on the surface:\n{}", error);
        }
        cr.scale(scale, scale);
        page.render(&cr);
    }
    /* Write Surface to file */
    // let mut file = File::create("output.png").expect("Couldn't create file.");
    // surface
    //     .write_to_png(&mut file)
    //     .expect("Failed to write file");

    debug!("Copying to clipboard");

    let byte_image_result = surface.data();
    if let Err(error) = byte_image_result {
        error!("Failed to access surface data:\n{}", error);
        return None;
    }
    let byte_image = byte_image_result.unwrap().to_vec();
    Some((byte_image, width, height))
}

fn on_clipboard_change(cb: &mut Clipboard) {
    let text = cb.get_text();
    if text.is_err() {
        debug!("Something happened");
        return;
    }

    let text = text.unwrap();
    debug!("Clipboard changed to: {}", text);
    if !has_latex_macro(&text) {
        return;
    }

    let snippet = snip_latex(&text);
    let latex_code = string_in_template(&snippet);
    match tectonic_rendering(&latex_code) {
        Some((raw_bytes, width, height)) => {
            let test_image = ImageData {
                width: width as usize,
                height: height as usize,
                bytes: Cow::from(raw_bytes),
            };

            if let Err(error) = cb.set_image(test_image) {
                debug!("Failed to push image in to clipboard {}", error);
                return;
            }

            if let Err(error) = Notification::new()
                .summary("Clip Tex")
                .body("Ready to Paste")
                .image_path("ClipTex.png")
                .show()
            {
                error!("For some reason i couldn't notify you but your compilation is done and in your clipboard!:\n{}",error);
                return;
            }
        }
        _ => error!("Failed to compile the code but just continueing as nothing happend =)"),
    }
}

#[cfg(target_os = "windows")]
fn main() {
    /* Init Logger */
    env_logger::init();
    println!("Started ClipTex");
    CKey.bind(|| {
        if LControlKey.is_pressed() {
            let mut cb = Clipboard::new().expect("couldn't create clipboard");
            on_clipboard_change(&mut cb);
        }
    });
    inputbot::handle_input_events();
}

struct LinuxHandler {
    cb: Clipboard,
}
impl LinuxHandler {
    fn new() -> LinuxHandler {
        LinuxHandler {
            cb: Clipboard::new().expect("Failed to create clipboard connection"),
        }
    }
}

impl ClipboardHandler for LinuxHandler {
    fn on_clipboard_change(&mut self) -> clipboard_master::CallbackResult {
        on_clipboard_change(&mut self.cb);
        CallbackResult::Next
    }
}

#[cfg(target_os = "linux")]
fn main() {
    /* Init Logger */
    env_logger::init();
    let mut master = clipboard_master::Master::new(LinuxHandler::new());
    master.run().expect("Clipboard Master failed you");
}
