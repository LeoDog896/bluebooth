use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt};
use relm4::{send, AppUpdate, Model, RelmApp, Sender, WidgetPlus, Widgets};

#[derive(Default)]
struct AppModel {
    counter: u8,
}

enum AppMsg {
    Increment,
    Decrement,
}

impl Model for AppModel {
    type Msg = AppMsg;
    type Widgets = AppWidgets;
    type Components = ();
}

impl AppUpdate for AppModel {
    fn update(&mut self, msg: AppMsg, _components: &(), _sender: Sender<AppMsg>) -> bool {
        match msg {
            AppMsg::Increment => {
                self.counter = self.counter.wrapping_add(1);
            }
            AppMsg::Decrement => {
                self.counter = self.counter.wrapping_sub(1);
            }
        }
        true
    }
}

#[relm4_macros::widget]
impl Widgets<AppModel, ()> for AppWidgets {
    view! {
        gtk::ApplicationWindow {
            set_title: Some("Bluebooth"),
            set_default_width: 300,
            set_child = Some(&gtk::Box) {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,
                set_spacing: 5,
				append = &gtk::Box {
					set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 10,
					append = &gtk::Label {
						set_margin_all: 5,
						set_label: watch! { &format!("No devices found") },
					},
					append = &gtk::Button {
						set_label: "Find New Devices",
						connect_clicked(sender) => move |_| {
							send!(sender, AppMsg::Increment);
						},
					}
				},
				append = &gtk::Box {
					set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 10,
				}
            },
        }
    }
}

fn main() {
    let model = AppModel::default();
    let app = RelmApp::new(model);
    app.run();
}