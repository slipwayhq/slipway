use slipway_lib::AppExecutionState;

mod to_view_model;

// • 0
// │ • 1
// │ │ • 2
// ├─│─│─• 5
// ├─│─│─│─• 6
// ╰─│─│─┼─│─• 4
//   │ │ ├─│─│─• 8
//   │ │ │ │ ╰─│─• 7
//   │ │ ╰─│───┴─┴─• 9
//   ╰─┴───┴───────┴─• 10
// • 3

fn print_app_state(state: &AppExecutionState<'_>) {
    let view_model = to_view_model::to_view_model(state);
}
