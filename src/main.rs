use arboard::{Clipboard, ImageData};
use cairo::ImageSurface;
use env_logger;
use inputbot::{KeySequence, KeybdKey::*, MouseButton::*};
use log::{debug, error, info, log_enabled, warn};
use notify_rust::Notification;
use std::{borrow::Cow, fs::File};
use std::{fs, process::Command};
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

fn tectonic_rendering(latex_code: &str) -> (Vec<u8>, i32, i32) {
    /* Render latex to PDF */
    let scale = 5.0;
    let mut pdf_vec = tectonic::latex_to_pdf(latex_code).expect("Couldn't compile Latex code");
    let bytes = pdf_vec.as_mut_slice();

    // let mut file = File::create("compare.pdf").expect("Couldn't create file");
    // file.write_all(bytes).expect("Couldn't write to pdf file");

    /* Parse PDF data with Poppler */
    let document = poppler::PopplerDocument::new_from_data(bytes, "").expect("Poppler failed");
    let page = document.get_page(0).expect("Getting page failed");
    let (width, height) = page.get_size();
    let width = (scale * width) as i32;
    let height = (scale * height) as i32;
    debug!("Width: {}, Height: {}", width, height);

    /* Create Cairo Context for rendering and Render PDF */
    let mut surface = ImageSurface::create(cairo::Format::ARgb32, width, height)
        .expect("Couldn't create cairo surface");
    let cr = cairo::Context::new(&surface).expect("Couldn't create cairo context");
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.paint().expect("Failed to paint");
    cr.scale(scale, scale);
    page.render(&cr);

    /* Write Surface to file */
    let mut file = File::create("output.png").expect("Couldn't create file.");
    surface
        .write_to_png(&mut file)
        .expect("Failed to write file");

    debug!("Copying to clipboard");
    surface.flush();
    let data = surface.data().expect("failed to get data");
    let byte_image = data.to_vec();
    (byte_image, width, height)
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
    let (raw_bytes, width, height) = tectonic_rendering(&latex_code);

    let test_image = ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::from(raw_bytes),
    };

    cb.set_image(test_image)
        .expect("Failed to push image in to clipboard");

    Notification::new()
        .summary("Clip Tex")
        .body("Ready to Paste")
        .image_path("ClipTex.png")
        .show()
        .expect("No ring ring, have you blocked me? :(");
}

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
    let _result = Command::new("xhost")
        .arg("si:localuser:root")
        .status()
        .expect("Failed to run xhost");
    ctrlc::set_handler(move || {
        let _result = Command::new("xhost")
            .arg("-si:localuser:root")
            .status()
            .expect("Failed to run xhost");
        std::process::exit(0);
    })
    .expect("Failed to set handler for ctrlc");
    inputbot::handle_input_events();
}
