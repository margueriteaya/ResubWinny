use std::collections::HashMap;

use serde::Serialize;

use crate::{RubyBinding, native_b24, scene_ruby_bindings};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RegionKey {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) is_ruby: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RegionInterval {
    pub(crate) begin_ms: i64,
    pub(crate) end_ms: i64,
    pub(crate) wait_duration_ms: i64,
    pub(crate) plane_width: i32,
    pub(crate) plane_height: i32,
    pub(crate) source_pid: Option<u16>,
    pub(crate) region: native_b24::CaptionRegion,
    pub(crate) characters: Vec<native_b24::CaptionCharacter>,
    pub(crate) drcs_glyphs: Vec<native_b24::DrcsGlyph>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ruby_binding: Option<RubyBinding>,
}

impl RegionInterval {
    pub(crate) fn key(&self) -> RegionKey {
        RegionKey {
            x: self.region.x,
            y: self.region.y,
            width: self.region.width,
            height: self.region.height,
            is_ruby: self.region.is_ruby,
        }
    }

    pub(crate) fn same_visual(&self, other: &Self) -> bool {
        self.plane_width == other.plane_width
            && self.plane_height == other.plane_height
            && self.key() == other.key()
            && self.characters == other.characters
            && self.drcs_glyphs == other.drcs_glyphs
    }
}

pub(crate) fn scene_intervals(scene: &native_b24::CaptionScene) -> Vec<RegionInterval> {
    let ruby_bindings = scene_ruby_bindings(scene);
    scene
        .regions
        .iter()
        .enumerate()
        .filter_map(|region| {
            let (region_index, region) = region;
            let start = region.first_character as usize;
            let end = start
                .saturating_add(region.character_count as usize)
                .min(scene.characters.len());
            let characters = scene.characters.get(start..end)?.to_vec();
            (!characters.is_empty()).then(|| {
                let drcs_glyphs = scene
                    .drcs_glyphs
                    .iter()
                    .filter(|glyph| {
                        characters
                            .iter()
                            .any(|character| character.drcs_code == glyph.drcs_code)
                    })
                    .cloned()
                    .collect();
                RegionInterval {
                    begin_ms: scene.pts_ms,
                    end_ms: scene.pts_ms,
                    wait_duration_ms: scene.wait_duration_ms,
                    plane_width: scene.plane_width,
                    plane_height: scene.plane_height,
                    source_pid: None,
                    region: region.clone(),
                    characters,
                    drcs_glyphs,
                    ruby_binding: ruby_bindings.get(&region_index).cloned(),
                }
            })
        })
        .collect()
}

pub(crate) fn caption_end(start_ms: i64, duration_ms: i64, next_start_ms: i64) -> i64 {
    if duration_ms > 0 && duration_ms != i64::MAX {
        start_ms.saturating_add(duration_ms)
    } else {
        next_start_ms
    }
}

fn close_region_interval(interval: &mut RegionInterval, fallback_end_ms: i64) {
    interval.end_ms = caption_end(
        interval.begin_ms,
        interval.wait_duration_ms,
        fallback_end_ms,
    )
    .max(interval.begin_ms.saturating_add(100));
}

pub(crate) fn apply_scene_intervals(
    active: &mut HashMap<RegionKey, RegionInterval>,
    scene: &native_b24::CaptionScene,
) -> Vec<RegionInterval> {
    let mut closed = Vec::new();
    let mut previous_active = std::mem::take(active);
    for interval in scene_intervals(scene) {
        let key = interval.key();
        match previous_active.remove(&key) {
            Some(previous) if previous.same_visual(&interval) => {
                active.insert(key, previous);
            }
            Some(mut previous) => {
                close_region_interval(&mut previous, scene.pts_ms);
                closed.push(previous);
                active.insert(key, interval);
            }
            None => {
                active.insert(key, interval);
            }
        }
    }
    for (_, mut previous) in previous_active {
        close_region_interval(&mut previous, scene.pts_ms);
        closed.push(previous);
    }
    closed
}

pub(crate) fn finish_scene_intervals(
    active: &mut HashMap<RegionKey, RegionInterval>,
    end_ms: i64,
) -> Vec<RegionInterval> {
    active
        .drain()
        .map(|(_, mut interval)| {
            close_region_interval(&mut interval, end_ms);
            interval
        })
        .collect()
}
