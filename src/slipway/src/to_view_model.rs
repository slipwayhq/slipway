use std::collections::HashMap;

use slipway_engine::{ComponentHandle, ComponentState, RigExecutionState};

pub(super) fn to_view_model<'state, 'rig, 'cache>(
    state: &'state RigExecutionState<'rig, 'cache>,
) -> RigExecutionStateViewModel<'rig>
where
    'state: 'rig,
{
    let mut groups = Vec::new();

    let mut used_shortcuts = HashMap::new();
    let mut all_output_row_indexes = HashMap::new();

    let components = &state.component_states;

    for (group_index, group) in state.component_groups.iter().enumerate() {
        let mut group_view_model = ComponentGroupViewModel {
            components: Vec::new(),
        };

        let mut row_index = 0;
        for &handle in state.valid_execution_order.iter() {
            if !group.contains(handle) {
                continue;
            }

            let state = components.get(handle).expect("Component should exist");

            let shortcut = to_shortcut(handle, &mut used_shortcuts);

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

        groups.push(group_view_model);
    }

    for group_view_models in groups.iter_mut() {
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

    RigExecutionStateViewModel { groups }
}

pub(super) fn to_shortcuts<'rig>(
    state: &RigExecutionState<'rig, '_>,
) -> HashMap<String, &'rig ComponentHandle> {
    let mut shortcuts = HashMap::new();
    for &handle in state.valid_execution_order.iter() {
        to_shortcut(handle, &mut shortcuts);
    }
    shortcuts
}

fn to_shortcut<'rig>(
    handle: &'rig ComponentHandle,
    used_shortcuts: &mut HashMap<String, &'rig ComponentHandle>,
) -> String {
    let mut s = String::new();
    for c in handle.0.chars() {
        s.push(c);
        if !used_shortcuts.contains_key(&s) {
            used_shortcuts.insert(s.clone(), handle);
            break;
        }
    }
    s
}

pub(super) struct RigExecutionStateViewModel<'rig> {
    pub groups: Vec<ComponentGroupViewModel<'rig>>,
}

pub(super) struct ComponentGroupViewModel<'rig> {
    pub components: Vec<ComponentViewModel<'rig>>,
}

pub(super) struct ComponentViewModel<'rig> {
    pub handle: &'rig ComponentHandle,
    pub state: &'rig ComponentState<'rig>,
    pub shortcut: String,
    pub group_index: usize,
    pub row_index: usize,
    pub input_columns_indexes: Vec<usize>,
    pub output_row_indexes: Vec<usize>,
    // row: Vec<ComponentRowCharacter>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use common_macros::slipway_test_async;
    use serde_json::json;
    use slipway_engine::{
        utils::ch, BasicComponentCache, ComponentRigging, Rig, RigSession, Rigging,
    };

    fn get_component<'rig>(
        view_model: &'rig RigExecutionStateViewModel<'rig>,
        handle: ComponentHandle,
    ) -> &'rig ComponentViewModel<'rig> {
        view_model
            .groups
            .iter()
            .flat_map(|g| g.components.iter())
            .find(|c| c.handle == &handle)
            .expect("Component should exist")
    }

    #[slipway_test_async]
    async fn it_should_generate_sensible_shortcuts() {
        let rig = Rig::for_test(Rigging {
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

        let component_cache = BasicComponentCache::for_test_permissive(&rig).await;
        let rig_session = RigSession::new(rig, &component_cache);
        let state = rig_session.initialize().unwrap();
        let view_model = to_view_model(&state);

        assert_eq!(view_model.groups.len(), 1);
        assert_eq!(get_component(&view_model, ch("cat")).shortcut, "c");
        assert_eq!(get_component(&view_model, ch("coat")).shortcut, "co");
        assert_eq!(get_component(&view_model, ch("coal")).shortcut, "coa");
        assert_eq!(get_component(&view_model, ch("coast")).shortcut, "coas");
        assert_eq!(get_component(&view_model, ch("dog")).shortcut, "d");
    }

    #[slipway_test_async]
    async fn it_should_set_group_and_row_indexes() {
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
        let rig = Rig::for_test(Rigging {
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

        let component_cache = BasicComponentCache::for_test_permissive(&rig).await;
        let rig_session = RigSession::new(rig, &component_cache);
        let state = rig_session.initialize().unwrap();
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

    #[slipway_test_async]
    async fn it_should_set_input_and_output_indexes() {
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
        let rig = Rig::for_test(Rigging {
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

        let component_cache = BasicComponentCache::for_test_permissive(&rig).await;
        let rig_session = RigSession::new(rig, &component_cache);
        let state = rig_session.initialize().unwrap();
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
