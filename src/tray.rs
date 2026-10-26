use crate::engine::Request;
use crate::i18n::tr as t;
use std::sync::mpsc::Sender;

pub struct Tray {
    pub tx: Sender<Request>,
    pub streaming: bool,
}

impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "elgato-virtualcam".into()
    }
    fn title(&self) -> String {
        t(if self.streaming {
            "Facecam Hub — câmera virtual ativa"
        } else {
            "Facecam Hub"
        })
    }
    fn icon_name(&self) -> String {
        "camera-web".into()
    }
    fn activate(&mut self, _: i32, _: i32) {
        let _ = self.tx.send(Request::Show);
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;
        vec![
            StandardItem {
                label: t("Abrir Facecam Hub"),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(Request::Show);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: t(if self.streaming {
                    "Parar câmera virtual"
                } else {
                    "Iniciar câmera virtual"
                }),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(if this.streaming {
                        Request::Stop
                    } else {
                        Request::Start { preview: false }
                    });
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: t("Sair"),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(Request::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub fn spawn(tx: Sender<Request>) -> Result<ksni::blocking::Handle<Tray>, ksni::Error> {
    use ksni::blocking::TrayMethods;
    Tray {
        tx,
        streaming: false,
    }
    .spawn()
}
