//! Command-line editing actions: typeahead suggestion bookkeeping, history,
//! quick-search trigger.

use farx_core::Action;

use super::super::App;

impl App {
    pub(in crate::app) fn dispatch_cmdline(&mut self, action: &Action) -> bool {
        match action {
            Action::QuickSearch(ch) => self.active_panel_mut().enter_quick_search(*ch),
            Action::QuickSearchClear => self.active_panel_mut().clear_quick_search(),
            Action::CommandLineInput(ch) => {
                self.command_line.input_char(*ch);
                self.command_line.last_typed_tick = self.tick_count;
                self.update_slash_suggestions();
            }
            Action::CommandLineBackspace => {
                self.command_line.last_typed_tick = self.tick_count;
                self.command_line.backspace();
                self.update_slash_suggestions();
            }
            Action::CommandLineExecute => {
                self.slash_suggestions = None;
                self.smart_execute_command();
            }
            Action::CommandLineHistoryUp => self.command_line.history_up(),
            Action::CommandLineHistoryDown => self.command_line.history_down(),
            Action::CommandLineClear => {
                self.command_line.clear();
                self.slash_suggestions = None;
            }
            _ => return false,
        }
        true
    }
}
