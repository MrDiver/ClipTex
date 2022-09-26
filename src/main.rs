use arboard::{Clipboard, ImageData};
use clipboard_master::{CallbackResult, ClipboardHandler, Master};
use image::io::Reader as ImageReader;
use latex::{Document, DocumentClass};
use notify_rust::Notification;
use std::{
    borrow::Cow,
    env,
    fs::{create_dir_all, File},
    io::{self, Write},
    path::PathBuf,
    process::Command,
};

struct Handler {
    clipboard: Clipboard,
}

impl Handler {
    fn new() -> Handler {
        Handler {
            clipboard: Clipboard::new().unwrap(),
        }
    }

    fn validate_latex(input: String) -> Result<String, ()> {
        if input.starts_with("!tex ") {
            Ok(input.replace("!tex ", ""))
        } else {
            Err(())
        }
    }

    fn string2latex(input: &str) -> String {
        let mut doc = Document::new(DocumentClass::Other(String::from("standalone")));
        doc.push(input);
        let mut rendered =
            latex::print(&doc).expect("Something went wrong while compiling Latex! Try again!");
        rendered.insert_str(14, "[convert={density=300}, border=2mm]");
        rendered
    }

    fn write_to_file_and_copy(self: &mut Self, input: String) {
        /* Create a temporary file */
        let filename = PathBuf::from("ClipTex.tex");
        let foldername = PathBuf::from("clip_tex");

        /* Create temp directory to work in */
        let mut dir = env::temp_dir();
        dir.push(foldername);
        create_dir_all(&dir).expect("Creating directories failed miserably ... what a shame");
        env::set_current_dir(&dir).expect("Stripping filename went wrong");
        // println!("Tmp Dir: {}", dir.display());

        /* Write Latex to the file */
        let file_path = dir.join(&filename);
        // println!("Trying to open: {}", file_path.display());
        let mut tmp_file =
            File::create(&file_path).expect("Something went wrong while openging the file");
        write!(tmp_file, "{}", input).expect("Something went wrong while writing the file");

        /* Compile Latex File */
        println!(
            "New Working Directory: {}",
            env::current_dir().expect("No working directory?").display()
        );
        let args: Vec<String> = env::args().collect();
        let _exit_status = Command::new("latexmk")
            .arg(file_path.to_str().unwrap())
            .arg("-quiet")
            .arg("-gg")
            .arg(&args.get(1).unwrap_or(&String::from("latex")))
            // .arg("-dvi")
            .arg("-shell-escape")
            // .arg(format!("-output-directory={}", dir.to_str().unwrap()))
            .output()
            .expect("Command could not be ran :( poor latex");

        /* Convert dvi to png */
        // let file_path = dir.join(PathBuf::from("ClipTex.dvi"));
        // let exit_status = Command::new("dvipng")
        //     .arg(file_path.to_str().unwrap())
        //     .arg("-D 300")
        //     .arg("-oClipTex.png")
        //     .status().expect("Converting to png seems rather difficult to me. Im sorry i failed you :/ - with love dvipng");

        /* Copy to clipboard https://stackoverflow.com/questions/41034635/how-do-i-convert-between-string-str-vecu8-and-u8 */
        let img = ImageReader::open("ClipTex.png")
            .expect("Could not read image :( ")
            .decode()
            .expect("Decode Image failed");
        // img.as_bytes()
        let rgba8img = img.to_rgba8();
        let bytes = rgba8img.as_raw().as_slice();
        let test_image = ImageData {
            width: img.width() as usize,
            height: img.height() as usize,
            bytes: Cow::from(bytes),
        };

        self.clipboard
            .set_image(test_image)
            .expect("Failed to push image in to clipboard");

        Notification::new()
            .summary("Clip Tex")
            .body("Ready to Paste")
            .image_path("ClipTex.png")
            .show()
            .expect("No ring ring, have you blocked me? :(");
    }

    fn handle_content(self: &mut Self, input: String) {
        let result = Handler::validate_latex(input);
        if result.is_ok() {
            let latex_code = Handler::string2latex(&result.unwrap());
            // println!("=====================\nCompiled Code\n{}", latex_code);
            self.write_to_file_and_copy(latex_code)
        } else {
            // println!("This is not Latex!")
        }
    }
}

impl ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        // println!("Clipboard change happened!");

        let result = self.clipboard.get_text();
        match result {
            Ok(content) => self.handle_content(content),
            Err(_) => (), //println!("Whoops this was not Text it was an Image"),
        };
        CallbackResult::Next
    }

    fn on_clipboard_error(&mut self, error: io::Error) -> CallbackResult {
        eprintln!("Error: {}", error);
        CallbackResult::Next
    }
}

fn main() {
    let _ = Master::new(Handler::new()).run();
}
