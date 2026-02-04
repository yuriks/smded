use crate::room::{Room, RoomIndex, RoomRef};
use crate::tileset::{Tileset, TilesetIndex, TilesetKind, TilesetRef};
use crate::{room, smart_xml, tileset};
use slotmap::SlotMap;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Default)]
pub struct ProjectData {
    pub tilesets: SlotMap<TilesetRef, Tileset>,
    pub tileset_ids: BTreeMap<TilesetIndex, TilesetRef>,
    pub cre_tileset_ids: BTreeMap<TilesetIndex, TilesetRef>,

    pub rooms: SlotMap<RoomRef, Room>,
    pub room_ids: BTreeMap<RoomIndex, RoomRef>,
}

pub fn validate_smart_project_path(project_path: &Path) -> Result<(), String> {
    if !project_path.is_dir() {
        return Err("Not a directory".into());
    }
    if !project_path.join("project.xml").exists() {
        return Err("Does not contain project.xml".into());
    }
    if !project_path.join("Export").exists() {
        return Err("Does not contain a Export/ directory".into());
    }

    Ok(())
}

pub fn load_smart_project(project_path: &Path) -> anyhow::Result<ProjectData> {
    let mut project = ProjectData::default();

    let smart_tilesets = smart_xml::load_project_tilesets(project_path)?;
    for (index, tileset) in smart_tilesets.sce {
        // TODO encapsulate the combination of SlotMap + BTreeMap for index
        let tileset_ref = project.tilesets.try_insert_with_key(|handle| {
            tileset::load_from_smart(TilesetKind::Sce, index, tileset, handle)
        })?;
        project.tileset_ids.insert(index, tileset_ref);
    }
    for (index, tileset) in smart_tilesets.cre {
        // TODO encapsulate the combination of SlotMap + BTreeMap for index
        let tileset_ref = project.tilesets.try_insert_with_key(|handle| {
            tileset::load_from_smart(TilesetKind::Cre, index, tileset, handle)
        })?;
        project.cre_tileset_ids.insert(index, tileset_ref);
    }

    let smart_rooms = smart_xml::load_project_rooms(project_path)?;
    for (index, (room_name, room)) in smart_rooms {
        let room_ref = project
            .rooms
            .try_insert_with_key(|handle| room::load_from_smart(index, room_name, room, handle))?;
        project.room_ids.insert(index, room_ref);
    }

    Ok(project)
}
