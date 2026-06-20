use std::io::{Read, Write};
use std::net::UdpSocket;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use ink_in_space::{Trajectory, WritingPlane};
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use nalgebra::{Quaternion, Vector3};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum Button {
    Tip,
    Front,
    Middle,
    Rear,
}

#[repr(transparent)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct Buttons(u8);

impl Buttons {
    /// Returns how pressed a button currently is on a normalised 0-1 scale
    fn value(&self, button: Button) -> f32 {
        match button {
            Button::Tip => ((self.0 & 0b11100000) >> 5) as f32 / 0b111 as f32,
            Button::Front => ((self.0 & 0b00010000) >> 4) as f32,
            Button::Middle => ((self.0 & 0b00001110) >> 1) as f32 / 0b111 as f32,
            Button::Rear => (self.0 & 0b00000001) as f32,
        }
    }

    // Returns whether the given button is currently pressed
    fn pressed(&self, button: Button) -> bool {
        self.value(button) > 0.0
    }

    fn active(&self) -> bool {
        self.pressed(Button::Tip) || self.pressed(Button::Middle)
    }
}

/// packet
/// +32: "position.x"
/// +32: "position.y"
/// +32: "position.z"
/// +32: "rotation.x"
/// +32: "rotation.y"
/// +32: "rotation.z"
/// +32: "rotation.w"
/// +3: "tip"
/// +1: "front"
/// +3: "middle"
/// +1: "rear"
#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Pen {
    position: Vector3<f32>,
    rotation: Quaternion<f32>,
    /// ttt|f|mmm|r
    buttons: Buttons,
    /// THIS FIELD IS NOT PART OF THE NETWORK PROTOCOL
    _padding: [u8; 3],
}

impl Pen {
    fn active(&self) -> bool {
        self.buttons.active()
    }
}

fn main() -> Result<(), Box<dyn ::std::error::Error>> {
    println!("Loading...");
    let mut det = Command::new("python")
        .arg("main.py")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()?;
    det.stdout.as_mut().unwrap().read_exact(&mut [0; 1])?;
    let det = Arc::new(Mutex::new(det));

    let socket = UdpSocket::bind("0.0.0.0:1273")?;

    let width = 800;
    let height = 800;

    let mut window = Window::new("Ink in Space", width, height, WindowOptions::default()).unwrap();
    let mut frame: Vec<u32> = vec![0xFFFFFF; width * height];

    let mut pred = Trajectory::new();

    let (tx, rx) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut buf = [0; size_of::<Pen>()];
        while socket.recv(&mut buf[..size_of::<Pen>() - 3]).is_ok() {
            let pen = *bytemuck::from_bytes::<Pen>(&buf);
            let _ = tx.try_send(pen);
        }
    });

    let mut prev = Pen::default();
    let mut rotation = 0.0;

    while window.is_open() {
        if let Ok(pen) = rx.try_recv() {
            // Draw
            if pen.active() {
                if !prev.active() {
                    pred.start_stroke();
                }

                pred.push(pen.position, pen.rotation);
            }

            // Undo
            if pen.buttons.pressed(Button::Front) && !prev.buttons.pressed(Button::Front) {
                pred.undo();
            }

            // --------------------------------------------------------------------------

            frame.fill(0xFFFFFF);

            if window.is_key_pressed(Key::Left, KeyRepeat::Yes) {
                rotation -= 1.0;
            }
            if window.is_key_pressed(Key::Right, KeyRepeat::Yes) {
                rotation += 1.0;
            }

            // Render stroke
            let radius: isize = 4 / 2;
            let (points, fc) = pred.normalise(rotation, WritingPlane::Auto);
            for p in points.iter().skip(fc) {
                let cx = ((p.x + 1.0) * 0.5 * (width - 1) as f32).round() as isize;
                let cy = ((1.0 - (p.y + 1.0) * 0.5) * (height - 1) as f32).round() as isize;

                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        if dx * dx + dy * dy > radius * radius {
                            continue;
                        }

                        let x = cx + dx;
                        let y = cy + dy;

                        if x >= 0 && x < width as isize && y >= 0 && y < height as isize {
                            frame[y as usize * width + x as usize] = 0x000000;
                        }
                    }
                }
            }

            // Render buttons
            for i in 0..u8::BITS {
                let bit = (pen.buttons.0 >> (u8::BITS - i - 1)) & 1;

                if bit == 1 {
                    let x0 = i * width as u32 / u8::BITS;
                    let x1 = (i + 1) * width as u32 / u8::BITS;

                    for y in 0..10.min(height as u32) {
                        for x in x0..x1 {
                            frame[(y * width as u32 + x) as usize] = 0x000000;
                        }
                    }
                }
            }

            // --------------------------------------------------------------------------

            // Submit
            if pen.buttons.pressed(Button::Rear) && !prev.buttons.pressed(Button::Rear) {
                let data = frame.clone();
                let det = det.clone();
                thread::spawn(move || {
                    use image::RgbImage;
                    let mut det = det.lock().unwrap();

                    fn vec_0rgb_to_image(mut pixels: Vec<u32>, width: u32, height: u32) -> RgbImage {
                        const SKIP_ROWS: u32 = 10;
                        const BACKGROUND: u32 = 0xFFFFFF;

                        let start = (SKIP_ROWS * width) as usize;
                        pixels = pixels[start..].to_vec();
                        let new_height = height - SKIP_ROWS;

                        let mut min_x = width;
                        let mut min_y = new_height;
                        let mut max_x = 0;
                        let mut max_y = 0;

                        for (i, &p) in pixels.iter().enumerate() {
                            if (p & 0xFFFFFF) != BACKGROUND {
                                let x = (i as u32) % width;
                                let y = (i as u32) / width;

                                min_x = min_x.min(x);
                                min_y = min_y.min(y);
                                max_x = max_x.max(x);
                                max_y = max_y.max(y);
                            }
                        }

                        let crop_w = max_x - min_x + 1;
                        let crop_h = max_y - min_y + 1;
                        let pad: u32 = 20;
                        let padded_w = crop_w + pad * 2;
                        let padded_h = crop_h + pad * 2;

                        let mut buf = vec![0xFFu8; (padded_w * padded_h * 3) as usize];
                        for y in min_y..=max_y {
                            for x in min_x..=max_x {
                                let idx = (y * width + x) as usize;
                                let p = pixels[idx];
                                let dst_x = (x - min_x + pad) as usize;
                                let dst_y = (y - min_y + pad) as usize;
                                let dst = (dst_y * padded_w as usize + dst_x) * 3;
                                buf[dst] = ((p >> 16) & 0xFF) as u8;
                                buf[dst + 1] = ((p >> 8) & 0xFF) as u8;
                                buf[dst + 2] = (p & 0xFF) as u8;
                            }
                        }

                        RgbImage::from_raw(padded_w, padded_h, buf).unwrap()
                    }

                    vec_0rgb_to_image(data, width as u32, height as u32).save("draw.png").unwrap();

                    det.stdin.as_ref().unwrap().write_all(b"draw.png\n").unwrap();
                    det.stdin.as_ref().unwrap().flush().unwrap();

                    let mut buf = vec![0; 4096];
                    let read = det.stdout.as_mut().unwrap().read(&mut buf).unwrap();

                    println!("{}", String::from_utf8_lossy(&buf[..read]).trim());
                });

                pred.reset();
            }

            // --------------------------------------------------------------------------

            prev = pen;
            window.update_with_buffer(&frame, width, height).unwrap();
        } else {
            window.update();
        }
    }

    Ok(())
}
