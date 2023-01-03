use iced::widget::image::Handle;
use iced::widget::{button, container, image, text, Column, Row};
use iced::window::Icon;
use iced::{
    executor, Alignment, Application, Command, Element, Executor, Length, Renderer, Settings, Theme,
};
use log::info;
use rust_embed::{EmbeddedFile, RustEmbed};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::Add;
use std::path::PathBuf;

mod env;

#[derive(RustEmbed)]
#[folder = "src/asset"]
struct Asset;

fn get_icon_file() -> EmbeddedFile {
    let file = Asset::get("Fries.png").unwrap();
    file
}

fn main() {
    env::setup_logger().unwrap();
    let mut settings = Settings::default();
    settings.window.size = (1000, 640);
    let cow = get_icon_file().data;
    settings.window.icon = Some(Icon::from_file_data(&*cow, None).unwrap());
    TrayMat::run(settings).unwrap();
}

#[derive(Deserialize, Serialize, Debug, Default)]
struct TrayMat {
    images: Vec<Wallpaper>,
    position: usize,
}

#[derive(Debug, Clone)]
enum Message {
    Loading,
    Loaded(Result<(Vec<Wallpaper>, usize), Error>),
    LoadError,
    NextMessage,
    LastMessage,
    SetWallpaper,
    Setting(Result<(), Error>),
}

#[derive(Debug, Clone)]
enum Error {
    ApiError,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
struct BingInfo {
    images: Vec<Wallpaper>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
struct Wallpaper {
    url: String,
    #[serde(rename(deserialize = "startdate"))]
    start_date: String,
}

impl Application for TrayMat {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            TrayMat::default(),
            Command::perform(Wallpaper::get_bing_info(0), Message::Loaded),
        )
    }

    fn title(&self) -> String {
        "TrayMat".into()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Loading => Command::none(),
            Message::Loaded(Err(_err)) => Command::none(),
            Message::Loaded(Ok((images, position))) => {
                *self = TrayMat { images, position };
                Command::none()
            }
            Message::NextMessage => {
                let len = self.images.len();
                let new_position = self.position + 1;
                if new_position < len {
                    self.position = new_position;
                }
                Command::none()
            }
            Message::LastMessage => {
                if self.position > 0 {
                    let new_position = self.position - 1;
                    if new_position.ge(&1) {
                        self.position = new_position;
                    }
                }
                Command::none()
            }
            Message::LoadError => Command::none(),
            Message::SetWallpaper => {
                let index = self.position;
                let wallpaper = self.images[index].clone();
                Command::perform(Wallpaper::set_wallpaper(wallpaper), Message::Setting)
            }
            Message::Setting(_) => Command::none(),
        }
    }

    fn view(&self) -> Element<'_, Self::Message, Renderer<Self::Theme>> {
        let btn_width = Length::Units(110);
        if self.images.len() > 0 {
            let handle = Wallpaper::get_image_handle(&self.images, self.position).unwrap();
            let image_viewer = image::viewer(handle);
            let image_content = Row::new()
                .push(image_viewer)
                .spacing(20)
                .align_items(Alignment::Center);
            let next_btn = button("Next Image")
                .padding(10)
                .width(btn_width)
                .on_press(Message::NextMessage);
            let set_btn = button("Set Wallpaper")
                .padding(10)
                .width(Length::Units(150))
                .on_press(Message::SetWallpaper);
            let last_btn = button("Perv Image")
                .padding(10)
                .width(btn_width)
                .on_press(Message::LastMessage);
            let btn_row = Row::new()
                .push(next_btn)
                .push(set_btn)
                .push(last_btn)
                .spacing(20)
                .align_items(Alignment::Fill);

            let content = Column::new()
                .push(image_content)
                .push(btn_row)
                .max_width(900)
                .spacing(20)
                .align_items(Alignment::Center);
            info!("start rendering");
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y()
                .into()
        } else {
            let content = Column::new()
                .push(text("searching wallpaper"))
                .width(Length::Shrink);
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y()
                .into()
        }
    }
}

impl Wallpaper {
    async fn get_bing_info(position: usize) -> Result<(Vec<Wallpaper>, usize), Error> {
        info!("start get bing info");
        let bing_api = "https://cn.bing.com/HPImageArchive.aspx?format=js&idx=0&n=8&mkt=zh-CN";
        let resp = reqwest::get(bing_api).await.expect("req api error");
        let bing_info: BingInfo = resp
            .json::<BingInfo>()
            .await
            .expect("deserialize data error");
        info!("bing_list:{:?}", bing_info);
        Ok((bing_info.images, position))
    }

    fn get_image_handle(wallpapers: &Vec<Wallpaper>, position: usize) -> Result<Handle, Error> {
        let wallpaper = &wallpapers[position];
        let url = &wallpaper.url;
        let date = &wallpaper.start_date;
        let buf = Self::download_image(url, date).unwrap();
        info!("start convert to handle");
        let file = fs::read(buf).unwrap();
        let handle = Handle::from_memory(file);
        info!("end convert to handle");
        Ok(handle)
    }

    async fn set_wallpaper(wallpaper: Wallpaper) -> Result<(), Error> {
        let path = Self::download_image(&wallpaper.url, &wallpaper.start_date).unwrap();
        let path = path.to_str().unwrap();
        wallpaper::set_from_path(path).expect("set wallpaper error");
        Ok(())
    }

    fn download_image(url: &str, date: &str) -> Result<PathBuf, Error> {
        let home_path = home::home_dir().expect("cant find home dir");
        let wallpaper_dir = home_path.join("Pictures").join("Wallpaper");
        info!("wallpaper_dir path:{:#?}", wallpaper_dir);

        if !wallpaper_dir.exists() {
            fs::create_dir_all(&wallpaper_dir).expect("create dir error");
        }
        let path = wallpaper_dir.join(format!("{}.jpg", date));
        info!("pic path {:#?}", &path);
        if !path.exists() {
            let bing_domain = "https://www.bing.com".to_string();
            let new_url = bing_domain.add(url.replace("1920x1080", "UHD").as_ref());
            let res = reqwest::blocking::get(new_url)?;
            let mut file = File::create(&path).unwrap();
            let stream = res.bytes()?;
            file.write_all(stream.as_ref()).unwrap();
        }
        Ok(path)
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Error {
        dbg!(error);
        Error::ApiError
    }
}
