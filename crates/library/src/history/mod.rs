mod export;
mod recorder;
pub mod store;
pub mod xspf;

pub use export::{export_document, HistoryExportFormat};
pub use recorder::{
    crossfader_gain, DeckPlaySnapshot, HistoryRecorder, HistoryRestorePrompt, HistorySettings,
};
pub use store::{history_dir_for_db, HistorySessionRow};
pub use xspf::{HistoryDocument, HistoryEntry};

use library_api::{EvtBody, Kind, Origin};
use library_core::{LibraryError, Result};
use std::path::Path;

use crate::LibraryManager;

impl LibraryManager {
    /// History recorder settings.
    pub fn set_history_settings(&mut self, settings: HistorySettings) {
        self.history.set_settings(settings);
    }

    pub fn history_settings(&self) -> &HistorySettings {
        self.history.settings()
    }

    pub fn history_restore_prompt(&self) -> Option<HistoryRestorePrompt> {
        self.history.restore_prompt().cloned()
    }

    pub fn history_tick(&mut self) -> Result<()> {
        self.history.tick(&self.db)?;
        self.emit_history_session_updated_if_dirty()
    }

    pub fn history_on_crossfader(&mut self, crossfader: f32) -> Result<()> {
        self.history.on_crossfader(&self.db, crossfader)?;
        self.emit_history_session_updated_if_dirty()
    }

    pub fn history_on_deck_updated(
        &mut self,
        deck_id: usize,
        snapshot: DeckPlaySnapshot,
    ) -> Result<()> {
        self.history.on_deck_updated(&self.db, deck_id, snapshot)?;
        self.emit_history_session_updated_if_dirty()
    }

    pub fn history_restore_session(&mut self, session_id: &str) -> Result<()> {
        self.history.restore_session(&self.db, session_id)?;
        self.emit_history_session_updated_if_dirty()
    }

    pub fn history_decline_restore(&mut self) -> Result<()> {
        self.history.decline_restore(&self.db)?;
        self.emit_history_session_updated_if_dirty()
    }

    pub fn history_new_session(&mut self) -> Result<()> {
        self.history.new_session(&self.db)?;
        self.emit_history_session_updated_if_dirty()
    }

    pub fn history_resume_session(&mut self) -> Result<()> {
        self.history.resume_session(&self.db)?;
        self.emit_history_session_updated_if_dirty()
    }

    pub fn history_can_resume(&self) -> bool {
        self.history.can_resume_session(&self.db)
    }

    pub fn list_history_sessions(&self) -> Result<Vec<HistorySessionRow>> {
        self.history.list_sessions(&self.db)
    }

    pub fn history_session_entries(&self, session_id: &str) -> Result<Vec<HistoryEntry>> {
        self.history.session_entries(&self.db, session_id)
    }

    pub fn rename_history_session(&mut self, session_id: &str, title: &str) -> Result<()> {
        self.history.rename_session(&self.db, session_id, title)?;
        self.emit_history_session_updated_if_dirty()
    }

    pub fn delete_history_session(&mut self, session_id: &str) -> Result<()> {
        self.history.delete_session(&self.db, session_id)?;
        self.emit_history_session_updated_if_dirty()
    }

    fn emit_history_session_updated_if_dirty(&mut self) -> Result<()> {
        if !self.history.take_session_updated() {
            return Ok(());
        }
        let session_id = self.history.active_session_id().map(|id| id.to_string());
        match self.publish_evt(
            Origin::Library,
            Kind::HistorySessionUpdated,
            EvtBody::HistorySessionUpdated { session_id },
        ) {
            Ok(()) => Ok(()),
            Err(err) => {
                self.history.mark_session_updated();
                Err(err)
            }
        }
    }

    pub fn history_dir(&self) -> &Path {
        self.history.history_dir()
    }

    pub fn export_history_session(
        &self,
        session_id: &str,
        format: HistoryExportFormat,
        dest: &Path,
    ) -> Result<()> {
        let row = crate::history::store::get_session(&self.db, session_id)?.ok_or_else(|| {
            LibraryError::Backend {
                backend: "history",
                message: "session not found".into(),
            }
        })?;
        let doc = crate::history::store::load_document(Path::new(&row.xspf_path))?;
        export_document(&doc, format, dest)
    }
}
