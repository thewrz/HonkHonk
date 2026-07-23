use iced::advanced::widget::operation::{Operation, Outcome, Scrollable};
use iced::widget::Id;
use iced::{Rectangle, Task, Vector};

use crate::app::Message;
use crate::settings::SettingId;

const CONTENT_ID: &str = "honkhonk-settings-content";

pub(crate) fn content_scroll_id() -> Id {
    Id::new(CONTENT_ID)
}

pub(super) fn row_id(setting: SettingId) -> Id {
    Id::new(setting_key(setting))
}

pub(crate) fn locate_setting_row(setting: SettingId) -> Task<Message> {
    iced::advanced::widget::operate(FindRowOffset::new(setting)).map(Message::SettingsRowLocated)
}

fn setting_key(setting: SettingId) -> &'static str {
    match setting {
        SettingId::RescanLibrary => "honkhonk-setting-rescan-library",
        SettingId::Theme => "honkhonk-setting-theme",
        SettingId::Density => "honkhonk-setting-density",
        SettingId::PanelAnimations => "honkhonk-setting-panel-animations",
        SettingId::MicPassthrough => "honkhonk-setting-mic-passthrough",
        SettingId::MicPassthroughLevel => "honkhonk-setting-mic-passthrough-level",
        SettingId::OverlapMode => "honkhonk-setting-overlap-mode",
        SettingId::MonitorDevice => "honkhonk-setting-monitor-device",
        SettingId::Renderer => "honkhonk-setting-renderer",
    }
}

struct FindRowOffset {
    scroll_id: Id,
    row_id: Id,
    content_y: Option<f32>,
    row_y: Option<f32>,
}

impl FindRowOffset {
    fn new(setting: SettingId) -> Self {
        Self {
            scroll_id: content_scroll_id(),
            row_id: row_id(setting),
            content_y: None,
            row_y: None,
        }
    }
}

impl Operation<f32> for FindRowOffset {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<f32>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        _bounds: Rectangle,
        content_bounds: Rectangle,
        _translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        if id == Some(&self.scroll_id) {
            self.content_y = Some(content_bounds.y);
        }
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if id == Some(&self.row_id)
            && let Some(content_y) = self.content_y
        {
            self.row_y = Some((bounds.y - content_y).max(0.0));
        }
    }

    fn finish(&self) -> Outcome<f32> {
        self.row_y.map_or(Outcome::None, Outcome::Some)
    }
}
