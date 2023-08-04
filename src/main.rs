pub mod window;

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::result;

use argh::FromArgs;

use iced::theme::Theme;
use iced::widget::{container, text, Column, Row, Space, Text};
use iced::{
    subscription, Application, Color, Command, Element, Event, Font, Length, Renderer, Settings,
    Subscription,
};

#[derive(FromArgs)]
/// Differ
struct Args {
    /// input file
    #[argh(positional)]
    file: String,
}

fn main() -> iced::Result {
    let args: Args = argh::from_env();

    let path: &Path = Path::new(&args.file);

    HexView::run(Settings {
        id: None,
        window: window::settings(),
        flags: Flags {
            file_path: path.to_path_buf(),
        },
        default_font: Font::DEFAULT,
        default_text_size: 16.0,
        antialiasing: false,
        exit_on_close_request: true,
    })
}

fn read_file(path: &Path) -> std::result::Result<BinFile, Error> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_error) => return result::Result::Err(Error::IOError),
    };

    let mut buf_reader = BufReader::new(file);
    let mut buffer = Vec::new();

    let _ = buf_reader
        .read_to_end(&mut buffer)
        .or(result::Result::Err(Error::IOError));

    println!(
        "Read {} bytes from {}",
        buffer.len(),
        path.file_name().unwrap().to_str().unwrap()
    );

    Ok(BinFile {
        file_name: path.file_name().unwrap().to_str().unwrap().to_string(),
        file_data: buffer,
    })
}

#[derive(Debug, Default)]
struct HexView {
    file: BinFile,
    cur_pos: usize,
}

#[derive(Debug, Clone)]
enum Message {
    FileLoaded(Result<BinFile, Error>),
    EventOccurred(Event),
}

struct Flags {
    file_path: PathBuf,
}

struct HexRow {
    offset: usize,
    data: Vec<u8>,
}

impl Application for HexView {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = Flags;

    fn new(_flags: Flags) -> (HexView, Command<Message>) {
        let path = _flags.file_path;
        let read_file_result = read_file(&path);

        (
            HexView {
                file: BinFile {
                    file_name: String::from("Loading"),
                    file_data: vec![],
                },
                cur_pos: 0,
            },
            Command::perform(async { read_file_result }, Message::FileLoaded),
        )
    }

    fn title(&self) -> String {
        String::from("BDiff")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::FileLoaded(Ok(bin_file)) => {
                *self = HexView {
                    file: bin_file,
                    cur_pos: 0,
                };
                Command::none()
            }
            Message::FileLoaded(Err(_error)) => Command::none(),
            Message::EventOccurred(event) => {
                if let Event::Mouse(iced::mouse::Event::WheelScrolled {
                    delta: iced::mouse::ScrollDelta::Lines { y, .. },
                }) = event
                {
                    self.cur_pos = (self.cur_pos as i32 - y as i32 * 0x10).max(0) as usize;
                }

                Command::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        subscription::events().map(Message::EventOccurred)
    }

    fn view(&self) -> Element<Message> {
        let content = {
            let file_name_text: Text = Text::new(self.file.file_name.clone())
                .font(Font::with_name("Calibri"))
                .size(24);

            let num_rows: u32 = 30;

            let mut i = 0;
            let mut cur_offset: usize = self.cur_pos;

            let mut rows = Vec::new();

            while i < num_rows && cur_offset + 0x10 < self.file.file_data.len() {
                let hex_row = HexRow {
                    offset: cur_offset,
                    data: self.file.file_data[cur_offset..cur_offset + 0x10].to_vec(),
                };

                rows.push(hex_row);
                cur_offset += 0x10;
                i += 1;
            }

            let mut row_elements: Vec<Element<Message, Renderer>> = rows
                .iter()
                .map(|row| {
                    let mut row_children: Vec<Element<Message, Renderer>> = Vec::new();

                    let offset_text: Element<Message> = Element::from(
                        text(format!(
                            "{:04X?} {:04X?}",
                            row.offset >> 0x10,
                            row.offset % 0x10000
                        ))
                        .font(Font::with_name("Consolas"))
                        .style(Color::from_rgb8(0x98, 0x98, 0x98)),
                    );

                    let mut hex_texts: Vec<Element<Message, Renderer>> = row
                        .data
                        .iter()
                        .map(|byte| {
                            let hex_color: Color = match *byte {
                                0 => Color::from_rgb8(0x80, 0x80, 0x80),
                                _ => Color::WHITE,
                            };
                            text(format!("{:02X?}", byte))
                                .font(Font::with_name("Consolas"))
                                .style(hex_color)
                        })
                        .map(Element::from)
                        .collect();

                    let mut ascii_texts: Vec<Element<Message, Renderer>> = row
                        .data
                        .iter()
                        .map(|byte| {
                            let ascii_char: char = match *byte {
                                32..=126 => *byte as char,
                                _ => '·',
                            };
                            let ascii_color: Color = match *byte {
                                0 => Color::from_rgb8(0x40, 0x40, 0x40),
                                32..=126 => Color::WHITE,
                                _ => Color::from_rgb8(0x80, 0x80, 0x80),
                            };
                            text(ascii_char)
                                .font(Font::with_name("Consolas"))
                                .style(ascii_color)
                        })
                        .map(Element::from)
                        .collect();

                    row_children.push(offset_text);
                    row_children.push(Element::from(Space::with_width(10)));
                    row_children.append(&mut hex_texts);
                    row_children.push(Element::from(Space::with_width(10)));
                    row_children.append(&mut ascii_texts);

                    Row::with_children(row_children)
                })
                .map(Element::from)
                .collect();

            row_elements.insert(0, Element::from(file_name_text));

            let hex_table = Column::with_children(row_elements);

            hex_table.max_width(700)
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

#[derive(Default, Debug, Clone)]
struct BinFile {
    file_name: String,
    file_data: Vec<u8>,
}

#[derive(Debug, Clone)]
enum Error {
    IOError,
}
