mod default;
mod scale;
mod slider;

pub use default::*;
pub use egui;
pub use scale::*;
pub use slider::*;

/// Options for rendering a value as a struct (i.e. draw all of its subfields)
#[derive(Default, Debug)]
pub struct InspectArgsStruct {
    pub header: Option<bool>,
    pub indent_children: Option<bool>,
}

impl From<InspectArgsDefault> for InspectArgsStruct {
    fn from(default_args: InspectArgsDefault) -> Self {
        Self {
            header: default_args.header,
            indent_children: default_args.indent_children,
        }
    }
}

/// Renders a struct (i.e. draw all of its subfields). Most traits are implemented by hand-written code, but this trait
/// is normally generated by putting `#[derive(Inspect)]` on a struct
pub trait InspectRenderStruct<T> {
    fn render(data: &T, label: &'static str, ui: &mut egui::Ui, args: &InspectArgsStruct);
    fn render_mut(
        data: &mut T,
        label: &'static str,
        ui: &mut egui::Ui,
        args: &InspectArgsStruct,
    ) -> bool;
}

#[rustfmt::skip]
#[macro_export]
macro_rules! debug_inspect_impl {
    ($t: ty) => {
        impl egui_inspect::InspectRenderDefault<$t> for $t {
            fn render(
                data: &$t,
                label: &'static str,
                ui: &mut egui_inspect::egui::Ui,
                _: &egui_inspect::InspectArgsDefault,
            ) {
                let d = data;
                if label == "" {
                    ui.label(format!("{:?}", d));
                } else {
                    ui.label(format!("{}: {:?}", label, d));
                }
            }

            fn render_mut(
                data: &mut $t,
                label: &'static str,
                ui: &mut egui_inspect::egui::Ui,
                _: &egui_inspect::InspectArgsDefault,
            ) -> bool {
                let d = data;
                if label == "" {
                    ui.label(format!("{:?}", d));
                } else {
                    ui.label(format!("{}: {:?}", label, d));
                }
                false
            }
        }
    };
}
