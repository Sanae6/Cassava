#[cfg(target_os = "linux")]
use tao::platform::unix::WindowExtUnix;
use tao::{event_loop::EventLoop, window::{Window, WindowBuilder}};
use wry::WebViewBuilder;
#[cfg(target_os = "linux")]
use wry::WebViewBuilderExtUnix;

fn main() {
	let event_loop = EventLoop::new();
	let window = WindowBuilder::new().build(&event_loop).unwrap();

	let builder = WebViewBuilder::new().with_url("https://google.com").with_devtools(true);

	#[cfg(not(target_os = "linux"))]
	let webview = builder.build(&window).unwrap();
	#[cfg(target_os = "linux")]
	let webview = builder.build_gtk(window.gtk_window()).unwrap();

	// let mut app = App::default();
	event_loop.run(|burger, _, cf| {
    
  });
}
#[derive(Default)]
struct App {
  window: Option<Window>,
  webview: Option<wry::WebView>,
}
