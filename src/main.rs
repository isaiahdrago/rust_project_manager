use eframe::egui;
use egui::{Color32, Pos2};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum TaskStatus {
    NotStarted,
    InProgress,
    Completed,
}

impl TaskStatus {
    fn all() -> [Self; 3] {
        [Self::NotStarted, Self::InProgress, Self::Completed]
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::NotStarted => "Not Started",
            Self::InProgress => "In Progress",
            Self::Completed => "Completed",
        }
    }

    fn color(self) -> Color32 {
        match self {
            Self::NotStarted => Color32::from_rgb(210, 65, 65),
            Self::InProgress => Color32::from_rgb(225, 190, 55),
            Self::Completed => Color32::from_rgb(70, 190, 95),
        }
    }
}

#[derive(Clone, Debug)]
struct Task {
    id: usize,
    title: String,
    notes: String,
    status: TaskStatus,
}

#[derive(Clone, Debug)]
struct Wisp {
    position: Pos2,
    velocity: egui::Vec2,
    color: Color32,
    age: f32,
    max_lifespan: f32,
    size: f32,
}

struct TaskApp {
    tasks: Vec<Task>,
    wisps: Vec<Wisp>,
    next_id: usize,
    new_task_title: String,
    task_notes: String,
    last_time: f64,
}

impl Default for TaskApp {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            wisps: Vec::new(),
            next_id: 1,
            new_task_title: String::new(),
            task_notes: String::new(),
            last_time: 0.0,
        }
    }
}

impl eframe::App for TaskApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let current_time = ui.ctx().input(|i| i.time);
        if self.last_time == 0.0 {
            self.last_time = current_time;
        }
        let delta_time = (current_time - self.last_time) as f32;
        self.last_time = current_time;

        if let Some(cursor_pos) = ui.ctx().input(|i| i.pointer.hover_pos()) {
            let velocity = ui.ctx().input(|i| i.pointer.velocity());
            if velocity.length() > 100.0 {
                use std::time::SystemTime;
                let seed = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos();

                let direction = velocity / velocity.length();
                let color = Color32::from_rgb(
                    (seed & 0xff) as u8,
                    ((seed >> 8) & 0xff) as u8,
                    ((seed >> 16) & 0xff) as u8,
                );

                let burst_seed = seed.wrapping_add(7919);
                let rand_x = ((burst_seed % 32) as f32 - 16.0) * 0.8;
                let rand_y = (((burst_seed >> 4) % 32) as f32 - 16.0) * 0.8;

                self.wisps.push(Wisp {
                    // Spawn substantially ahead of the cursor, with random spread.
                    position: cursor_pos
                        + direction * 45.0 * velocity / 1000.0
                        + egui::vec2(rand_x, rand_y),
                    velocity: egui::vec2(-velocity.x * 0.2 + rand_x, -velocity.y * 0.2 + rand_y),
                    color,
                    age: 0.0,
                    max_lifespan: 0.5,
                    size: 10.0,
                });
            }
        }

        for wisp in &mut self.wisps {
            wisp.position += wisp.velocity * delta_time;
            wisp.age += delta_time;
            // Gradually shrink wisps over time
            wisp.size = (wisp.size - delta_time * 5.0).max(0.0);
        }
        // Remove dead particles
        self.wisps.retain(|wisp| wisp.age < wisp.max_lifespan);

        let painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("wisps_layer"),
        ));
        for wisp in &self.wisps {
            // Fade out the transparency (Alpha) as it gets older
            let life_ratio = 1.0 - (wisp.age / wisp.max_lifespan);
            let alpha = (life_ratio * 255.0) as u8;

            let wisp_color = Color32::from_rgba_unmultiplied(
                wisp.color.r(),
                wisp.color.g(),
                wisp.color.b(),
                alpha,
            );

            painter.circle_filled(wisp.position, wisp.size, wisp_color);
        }

        // Continually trigger frames to keep simulation moving fluidly
        ui.ctx().request_repaint();

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).fill(egui::Color32::from_rgb(18, 35, 72)))
            .show(ui, |ui| {
                ui.visuals_mut().override_text_color = Some(Color32::BLACK);
                ui.heading(egui::RichText::new("Task Manager").color(Color32::WHITE));
                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("New task:").color(Color32::WHITE));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_task_title)
                            .hint_text("Task title")
                            .desired_width(180.0),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.task_notes)
                            .hint_text("Notes (optional)")
                            .desired_width(220.0),
                    );

                    if ui.button("Add task").clicked() && !self.new_task_title.trim().is_empty() {
                        self.tasks.push(Task {
                            id: self.next_id,
                            title: self.new_task_title.trim().to_owned(),
                            notes: self.task_notes.trim().to_owned(),
                            status: TaskStatus::NotStarted,
                        });
                        self.next_id += 1;
                        self.new_task_title.clear();
                        self.task_notes.clear();
                    }
                });

                ui.separator();
                ui.add_space(8.0);

                // Apply moves after all columns have been rendered
                // Avoid issues caused when list is being iterated over
                let mut pending_move: Option<(usize, TaskStatus)> = None;

                ui.columns(3, |columns| {
                    for (column_index, status) in TaskStatus::all().into_iter().enumerate() {
                        let column = &mut columns[column_index];
                        let column_color = status.color();
                        let column_frame = egui::Frame::group(column.style())
                            .fill(column_color)
                            .stroke(egui::Stroke::NONE)
                            .inner_margin(egui::Margin::same(12));
                        let column_response = column_frame.show(column, |column| {
                            column.set_min_size(egui::vec2(220.0, 400.0));
                            column.label(
                                egui::RichText::new(status.display_name())
                                    .strong()
                                    .color(Color32::BLACK),
                            );
                            column.separator();
                            column.add_space(5.0);

                            for task in self.tasks.iter().filter(|task| task.status == status) {
                                let task_id = task.id;
                                let title = task.title.clone();
                                let notes = task.notes.clone();

                                column.dnd_drag_source(
                                    egui::Id::new(("task", task_id)),
                                    task_id,
                                    |card| {
                                        egui::Frame::new()
                                            .fill(Color32::WHITE)
                                            .inner_margin(egui::Margin::same(8))
                                            .show(card, |card| {
                                                card.label(
                                                    egui::RichText::new(title)
                                                        .strong()
                                                        .color(Color32::BLACK),
                                                );
                                                if !notes.is_empty() {
                                                    card.label(
                                                        egui::RichText::new(notes)
                                                            .color(Color32::BLACK),
                                                    );
                                                }
                                            });
                                    },
                                );
                                column.add_space(6.0);
                            }
                        });

                        if let Some(task_id) =
                            column_response.response.dnd_release_payload::<usize>()
                        {
                            pending_move = Some((*task_id, status));
                        }
                    }
                });

                if let Some((task_id, new_status)) = pending_move {
                    if let Some(task) = self.tasks.iter_mut().find(|task| task.id == task_id) {
                        task.status = new_status;
                    }
                }
            });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Task Manager",
        native_options,
        Box::new(|_cc| Ok(Box::new(TaskApp::default()))),
    )
}
