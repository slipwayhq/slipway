use std::collections::{HashMap, HashSet};

use slipway_lib::{AppExecutionState, ComponentHandle, ComponentState};

pub(super) fn to_view_model<'app>(
    state: &'app AppExecutionState<'app>,
) -> AppExecutionStateViewModel<'app> {
    let mut result = AppExecutionStateViewModel { groups: Vec::new() };
    let mut used_shortcuts = HashSet::new();
    let mut all_output_row_indexes = HashMap::new();

    let components = state.component_states();

    for (group_index, group) in state.component_groups().iter().enumerate() {
        let mut group_view_model = ComponentGroupViewModel {
            components: Vec::new(),
        };

        let mut row_index = 0;
        for &handle in state.valid_execution_order() {
            if !group.contains(handle) {
                continue;
            }

            let state = components.get(handle).expect("Component should exist");

            let shortcut = {
                let mut s = String::new();
                for c in handle.0.chars() {
                    s.push(c);
                    if !used_shortcuts.contains(&s) {
                        used_shortcuts.insert(s.clone());
                        break;
                    }
                }
                s
            };

            for &dependency_handle in state.dependencies.iter() {
                all_output_row_indexes
                    .entry(dependency_handle)
                    .or_insert(vec![])
                    .push(row_index);
            }

            let view_model = ComponentViewModel {
                handle,
                state,
                shortcut,
                group_index,
                row_index,
                input_columns_indexes: state
                    .dependencies
                    .iter()
                    .map(|d| {
                        group_view_model
                            .components
                            .iter()
                            .find(|&c| c.handle == *d)
                            .expect("Input component should already exist in group")
                            .row_index
                    })
                    .collect(),
                output_row_indexes: vec![],
            };

            group_view_model.components.push(view_model);

            row_index += 1;
        }

        result.groups.push(group_view_model);
    }

    for group_view_models in result.groups.iter_mut() {
        for component_view_model in group_view_models.components.iter_mut() {
            if let Some(output_row_indexes) =
                all_output_row_indexes.remove(&component_view_model.handle)
            {
                component_view_model.output_row_indexes = output_row_indexes;
            }

            // Sort the indexes so that the order is deterministic.
            component_view_model.output_row_indexes.sort();
            component_view_model.input_columns_indexes.sort();
        }
    }

    result
}

pub(super) struct AppExecutionStateViewModel<'app> {
    pub groups: Vec<ComponentGroupViewModel<'app>>,
}

pub(super) struct ComponentGroupViewModel<'app> {
    pub components: Vec<ComponentViewModel<'app>>,
}

pub(super) struct ComponentViewModel<'app> {
    pub handle: &'app ComponentHandle,
    pub state: &'app ComponentState<'app>,
    pub shortcut: String,
    pub group_index: usize,
    pub row_index: usize,
    pub input_columns_indexes: Vec<usize>,
    pub output_row_indexes: Vec<usize>,
    // row: Vec<ComponentRowCharacter>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use slipway_lib::{utils::ch, App, AppSession, ComponentRigging, Rigging};

    use super::*;

    fn get_component<'app>(
        view_model: &'app AppExecutionStateViewModel<'app>,
        handle: ComponentHandle,
    ) -> &'app ComponentViewModel<'app> {
        view_model
            .groups
            .iter()
            .flat_map(|g| g.components.iter())
            .find(|c| c.handle == &handle)
            .expect("Component should exist")
    }

    #[test]
    fn it_should_generate_sensible_shortcuts() {
        let app = App::for_test(Rigging {
            components: [
                ComponentRigging::for_test("cat", None),
                ComponentRigging::for_test("coat", Some(json!({"a": "$$.cat"}))),
                ComponentRigging::for_test("coal", Some(json!({"a": "$$.coat"}))),
                ComponentRigging::for_test("coast", Some(json!({"a": "$$.coal"}))),
                ComponentRigging::for_test("dog", Some(json!({"a": "$$.coast"}))),
            ]
            .into_iter()
            .collect(),
        });

        let app_session = AppSession::from(app);
        let state = app_session.initialize().unwrap();
        let view_model = to_view_model(&state);

        assert_eq!(view_model.groups.len(), 1);
        assert_eq!(get_component(&view_model, ch("cat")).shortcut, "c");
        assert_eq!(get_component(&view_model, ch("coat")).shortcut, "co");
        assert_eq!(get_component(&view_model, ch("coal")).shortcut, "coa");
        assert_eq!(get_component(&view_model, ch("coast")).shortcut, "coas");
        assert_eq!(get_component(&view_model, ch("dog")).shortcut, "d");
    }

    #[test]
    fn it_should_set_group_and_row_indexes() {
        // Dependency graph:
        // a
        // |
        // b
        // |
        // c
        //
        // d
        // |
        // e
        //
        // f
        let app = App::for_test(Rigging {
            components: [
                ComponentRigging::for_test("a", None),
                ComponentRigging::for_test("b", Some(json!({"a": "$$.a"}))),
                ComponentRigging::for_test("c", Some(json!({"b": "$$.b"}))),
                ComponentRigging::for_test("d", None),
                ComponentRigging::for_test("e", Some(json!({"d": "$$.d"}))),
                ComponentRigging::for_test("f", None),
            ]
            .into_iter()
            .collect(),
        });

        let app_session = AppSession::from(app);
        let state = app_session.initialize().unwrap();
        let view_model = to_view_model(&state);

        assert_eq!(view_model.groups.len(), 3);

        let a = get_component(&view_model, ch("a"));
        let b = get_component(&view_model, ch("b"));
        let c = get_component(&view_model, ch("c"));
        let d = get_component(&view_model, ch("d"));
        let e = get_component(&view_model, ch("e"));
        let f = get_component(&view_model, ch("f"));

        assert_eq!(a.group_index, 0);
        assert_eq!(a.row_index, 0);

        assert_eq!(b.group_index, 0);
        assert_eq!(b.row_index, 1);

        assert_eq!(c.group_index, 0);
        assert_eq!(c.row_index, 2);

        assert_eq!(d.group_index, 1);
        assert_eq!(d.row_index, 0);

        assert_eq!(e.group_index, 1);
        assert_eq!(e.row_index, 1);

        assert_eq!(f.group_index, 2);
        assert_eq!(f.row_index, 0);
    }

    #[test]
    fn it_should_set_input_and_output_indexes() {
        // Dependency graph:
        // a
        // |\
        // b c
        // |/
        // d
        //
        // e
        // |
        // f
        //
        // g
        let app = App::for_test(Rigging {
            components: [
                ComponentRigging::for_test("a", None),
                ComponentRigging::for_test("b", Some(json!({"a": "$$.a"}))),
                ComponentRigging::for_test("c", Some(json!({"a": "$$.a"}))),
                ComponentRigging::for_test("d", Some(json!({"b": "$$.b", "c": "$$.c"}))),
                ComponentRigging::for_test("e", None),
                ComponentRigging::for_test("f", Some(json!({"e": "$$.e"}))),
                ComponentRigging::for_test("g", None),
            ]
            .into_iter()
            .collect(),
        });

        let app_session = AppSession::from(app);
        let state = app_session.initialize().unwrap();
        let view_model = to_view_model(&state);

        assert_eq!(view_model.groups.len(), 3);

        let a = get_component(&view_model, ch("a"));
        let b = get_component(&view_model, ch("b"));
        let c = get_component(&view_model, ch("c"));
        let d = get_component(&view_model, ch("d"));
        let e = get_component(&view_model, ch("e"));
        let f = get_component(&view_model, ch("f"));
        let g = get_component(&view_model, ch("g"));

        let empty = Vec::<usize>::new();

        assert_eq!(a.input_columns_indexes, empty);
        assert_eq!(a.output_row_indexes, vec![1, 2]);

        assert_eq!(b.input_columns_indexes, vec![0]);
        assert_eq!(b.output_row_indexes, vec![3]);

        assert_eq!(c.input_columns_indexes, vec![0]);
        assert_eq!(c.output_row_indexes, vec![3]);

        assert_eq!(d.input_columns_indexes, vec![1, 2]);
        assert_eq!(d.output_row_indexes, empty);

        assert_eq!(e.input_columns_indexes, empty);
        assert_eq!(e.output_row_indexes, vec![1]);

        assert_eq!(f.input_columns_indexes, vec![0]);
        assert_eq!(f.output_row_indexes, empty);

        assert_eq!(g.input_columns_indexes, empty);
        assert_eq!(g.output_row_indexes, empty);
    }
}
