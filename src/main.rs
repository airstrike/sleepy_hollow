use iced::Alignment::Center;
use iced::futures::channel::mpsc;
use iced::time::Duration;
use iced::widget::{
    button, center, column, container, image, responsive, row, stack, text, toggler,
};
use iced::{ContentFit, Element, Fill, Subscription, Task};
use sipper::{Never, Sipper, StreamExt, sipper};

use std::time::Instant;

mod filter;
mod sample;
mod simulator;

use sample::PngScreenshot;

pub fn main() -> iced::Result {
    iced::application("iced • shader downsampler", App::update, App::view)
        .subscription(App::subscription)
        .run()
}

#[derive(Debug, Clone)]
enum Command {
    RenderSample,
}

#[derive(Debug, Clone)]
enum Event {
    Connected(mpsc::Sender<Command>),
    RenderResult(PngScreenshot),
    Error(String),
}
pub enum Render {
    Success {
        image: PngScreenshot,
        duration: Duration,
    },
    Failed(String),
}

#[derive(Default)]
struct App {
    render: Option<Render>,
    queued: Option<Instant>,
    sender: Option<mpsc::Sender<Command>>,
    cubic: bool,
}

#[derive(Debug, Clone)]
enum Message {
    Render,
    ToggleCubic(bool),
    ChannelEvent(Event),
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Render => {
                if let Some(sender) = &mut self.sender {
                    self.queued = Some(Instant::now());
                    let _ = sender.try_send(Command::RenderSample);
                }
                Task::none()
            }
            Message::ToggleCubic(b) => {
                self.cubic = b;
                Task::none()
            }
            Message::ChannelEvent(event) => match event {
                Event::Connected(sender) => {
                    self.sender = Some(sender);
                    // Auto-trigger a render on startup
                    Task::perform(async {}, |_| Message::Render)
                }
                Event::RenderResult(screenshot_data) => {
                    if let Some(start_time) = self.queued.take() {
                        let duration = Duration::from_secs_f32(start_time.elapsed().as_secs_f32());
                        self.render = Some(Render::Success {
                            image: screenshot_data,
                            duration,
                        });
                    }
                    Task::none()
                }
                Event::Error(error) => {
                    self.queued = None;
                    self.render = Some(Render::Failed(error));
                    Task::none()
                }
            },
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::run(stream).map(Message::ChannelEvent)
    }

    fn image_element<'a>(&'a self) -> Element<'a, Message> {
        // show the rendered image if we have it, using the cubic filter if enabled
        match &self.render {
            Some(Render::Success { image, .. }) => responsive(move |size| {
                if self.cubic {
                    filter::cubic_filtered_image(
                        image.raw_data.to_vec(),
                        image.size,
                        iced::Size::new(size.width as u32, size.height as u32),
                    )
                } else {
                    let image_handle = image::Handle::from_bytes(image.png_data.clone());
                    iced::widget::image(image_handle)
                        .content_fit(ContentFit::Contain)
                        .width(size.width)
                        .into()
                }
            })
            .into(),
            _ => center(text("No image yet")).into(),
        }
    }

    fn view(&self) -> Element<Message> {
        let header = row![
            container(text("Headless Render Sample")).width(Fill),
            toggler(self.cubic)
                .label("Cubic downsampling")
                .on_toggle(Message::ToggleCubic),
            button("Generate").on_press(Message::Render)
        ]
        .padding([0, 20])
        .spacing(10)
        .align_y(Center);

        // Determine what to display based on current state
        let display_content = match (&self.render, &self.queued) {
            (None, None) => {
                Element::from(center(text("No renders yet. Press 'Generate'.").size(18)))
            }
            (_, Some(_)) => {
                // We're rendering, but show the previous render if available
                let rendering_msg = center(
                    container(text(format!("Rendering..")).size(18))
                        .style(|theme: &iced::Theme| {
                            container::background(
                                theme
                                    .extended_palette()
                                    .background
                                    .strong
                                    .color
                                    .scale_alpha(0.5),
                            )
                        })
                        .padding(20),
                );

                // If we have a previous render, show it with the rendering message
                if let Some(render) = &self.render {
                    match render {
                        Render::Success { image, duration } => {
                            let elapsed = duration.as_secs_f32();
                            let status = container(
                                text(format!(
                                    "Previous render: ({:.3}s - {}x{})",
                                    elapsed, image.size.width, image.size.height
                                ))
                                .size(12),
                            )
                            .align_right(Fill);

                            stack![
                                center(column![
                                    container(
                                        container(stack![self.image_element()])
                                            .width(Fill)
                                            .padding(10)
                                            .style(container::rounded_box),
                                    )
                                    .width(Fill)
                                    .height(600),
                                    status
                                ]),
                                rendering_msg
                            ]
                            .into()
                        }
                        Render::Failed(error) => stack![
                            center(text(format!("Previous error: {}", error)).size(18)),
                            rendering_msg
                        ]
                        .into(),
                    }
                } else {
                    rendering_msg.into()
                }
            }
            (Some(render), None) => {
                match render {
                    Render::Success { image, duration } => {
                        let elapsed = duration.as_secs_f32();
                        let status = container(
                            text(format!(
                                "Render completed! ({:.3}s - {}x{})",
                                elapsed, image.size.width, image.size.height
                            ))
                            .size(12),
                        )
                        .align_right(Fill);

                        center(column![
                            container(
                                container(stack![self.image_element()])
                                    .width(Fill)
                                    .padding(10)
                                    .style(container::rounded_box),
                            )
                            .width(Fill)
                            .height(600),
                            status
                        ])
                        .into()
                    }
                    Render::Failed(error) => center(text(format!("Error: {}", error)).size(18)),
                }
            }
            .into(),
        };

        container(
            container(column![header, display_content].spacing(20))
                .width(Fill)
                .padding(20)
                .center_x(Fill),
        )
        .width(Fill)
        .style(container::bordered_box)
        .into()
    }
}

fn stream() -> impl Sipper<Never, Event> {
    sipper(async move |mut event_sender| {
        let (command_sender, mut command_receiver) = mpsc::channel(100);

        let _ = event_sender.send(Event::Connected(command_sender)).await;

        // create a single simulator that we'll reuse across all renders
        let mut simulator = simulator::Simulator::new();

        loop {
            if let Some(command) = command_receiver.next().await {
                match command {
                    Command::RenderSample => {
                        println!("Processing sample render request");

                        let result = sample::render(&mut simulator);

                        match result {
                            Ok(screenshot_data) => {
                                println!("Render completed successfully");
                                let _ = event_sender
                                    .send(Event::RenderResult(screenshot_data))
                                    .await;
                            }
                            Err(e) => {
                                println!("Render failed: {}", e);
                                let _ = event_sender.send(Event::Error(e)).await;
                            }
                        }
                    }
                }
            }
        }
    })
}
