    /// Render the progress view
    fn render_progress_view(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Back to Main").clicked() {
                self.current_view = CurrentView::Main;
            }
            
            self.progress_view.ui(ui);
        });
    }
