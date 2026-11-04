//! Native BannerModel/BannerFlagModel geometry, ordered dye layers and chunk NBT.
use crate::{scene::{Block,BlockPos,HandcraftedScene,Scene},pack::ResourceId,mesh::{Atlas,ChunkMesh},lighting::SkyLight};
use minecraftoss_core::nbt::Tag;
use minecraftoss_player::inventory::ItemStack;
use glam::{Vec3,Mat4,Quat,DVec3};
use std::collections::BTreeMap;

pub fn is_banner(b:&Block)->bool {b.id.namespace=="minecraft" && b.id.path.ends_with("_banner")}
pub fn placed_tag(p:BlockPos,stack:&ItemStack)->Tag {
    let mut data=BTreeMap::from([("id".into(),Tag::String("minecraft:banner".into())),
        ("x".into(),Tag::Int(p.0)),("y".into(),Tag::Int(p.1)),("z".into(),Tag::Int(p.2))]);
    let layers=stack.components.as_ref().and_then(|c|c.get("minecraft:banner_patterns")).and_then(serde_json::Value::as_array);
    if let Some(layers)=layers {
        data.insert("patterns".into(),Tag::List(layers.iter().filter_map(|v|Some(Tag::Compound(BTreeMap::from([
            ("pattern".into(),Tag::String(v.get("pattern")?.as_str()?.into())),
            ("color".into(),Tag::String(v.get("color")?.as_str()?.into()))])))).collect()));
    }
    if let Some(name)=stack.components.as_ref().and_then(|c|c.get("minecraft:custom_name")) {
        data.insert("CustomName".into(),Tag::String(name.to_string()));
    }
    Tag::Compound(data)
}
pub fn dropped_stack(block:&Block,tag:Option<&Tag>)->ItemStack {
    let id=format!("minecraft:{}",block.id.path.replace("_wall_banner","_banner"));
    let mut stack=ItemStack::new(&id,1);stack.max=16;
    let mut components=serde_json::Map::new();
    if let Some(layers)=tag.and_then(|t|t.get("patterns")).and_then(Tag::as_list) {
        let layers:Vec<_>=layers.iter().filter_map(|p|Some(serde_json::json!({"pattern":p.get("pattern")?.as_str()?,"color":p.get("color")?.as_str()?}))).collect();
        if !layers.is_empty(){components.insert("minecraft:banner_patterns".into(),layers.into());}
    }
    if let Some(name)=tag.and_then(|t|t.get("CustomName")).and_then(Tag::as_str) {
        components.insert("minecraft:custom_name".into(),serde_json::from_str(name).unwrap_or_else(|_|name.into()));
    }
    if !components.is_empty(){stack.components=Some(components.into());}
    stack
}

fn cube(mesh:&mut ChunkMesh,atlas:&Atlas,texture:&str,pose:Mat4,from:[f32;3],to:[f32;3],uv:[f32;2],size:[f32;3],tint:[f32;3],light:[f32;2]) {
    let Ok(id)=ResourceId::parse(texture) else{return};
    if !atlas.contains(&id){return;}
    let start=mesh.vertices.len();
    // Reuse the native ModelPart.Cube UV layout, then replace its entity-space
    // basis with BannerRenderer's (x,-y,-z)/24 transformation.
    crate::cow_render::cube_scaled(mesh,DVec3::ZERO,Quat::IDENTITY,Vec3::ONE,atlas.entity_region(&id),light[0],light[1],from,to,uv,[0.0;3],Quat::IDENTITY,tint,[64.0;2],Some(size),false);
    for v in &mut mesh.vertices[start..] {
        let p=Vec3::from_array(v.position);
        v.position=pose.transform_point3(Vec3::new(-p.x*16.0,24.016-p.y*16.0,p.z*16.0)).to_array();
    }
}
/// Twenty by forty native texels, independent of the block collision/state palette.
/// No per-design texture allocation: all banners share the original mask atlas.
fn append(block:&Block,tag:Option<&Tag>,atlas:&Atlas,light:[f32;2],pose:Mat4,phase:f32,solid:&mut ChunkMesh,layers:&mut ChunkMesh) {
    let wall=block.id.path.ends_with("_wall_banner");
    let (top,z)=if wall{(-20.5,10.5)}else{(-44.0,0.0)};
    if !wall {cube(solid,atlas,"minecraft:entity/banner/banner_base",pose,[-1.0,-42.0,-1.0],[1.0,0.0,1.0],[44.0,0.0],[2.0,42.0,2.0],[1.0;3],light);}
    cube(solid,atlas,"minecraft:entity/banner/banner_base",pose,[-10.0,top,z-1.0],[10.0,top+2.0,z+1.0],[0.0,42.0],[20.0,2.0,2.0],[1.0;3],light);
    let flap=(-0.0125+0.01*(std::f32::consts::TAU*phase).cos())*std::f32::consts::PI;
    let flag=pose*Mat4::from_translation(Vec3::new(0.0,top,z))*Mat4::from_rotation_x(flap);
    let color=block.id.path.trim_end_matches("_banner").trim_end_matches("_wall");
    let tint=crate::item_icons::dye(color).map(|n|n as f32/255.0);
    cube(solid,atlas,"minecraft:entity/banner/base",flag,[-10.0,0.0,-2.0],[10.0,40.0,-1.0],[0.0;2],[20.0,40.0,1.0],tint,light);
    for (i,layer) in tag.and_then(|t|t.get("patterns")).and_then(Tag::as_list).into_iter().flatten().take(16).enumerate() {
        let Some(pattern)=layer.get("pattern").and_then(Tag::as_str) else{continue};
        let Ok(id)=ResourceId::parse(pattern) else{continue};
        let color=layer.get("color").and_then(Tag::as_str).unwrap_or("white");
        let tint=crate::item_icons::dye(color).map(|n|n as f32/255.0);
        let offset=(i+1) as f32*0.012;
        cube(layers,atlas,&format!("{}:entity/banner/{}",id.namespace,id.path),flag,[-10.0,0.0,-2.0-offset],[10.0,40.0,-1.0+offset],[0.0;2],[20.0,40.0,1.0],tint,light);
    }
}
pub fn append_world(scene:&HandcraftedScene,atlas:&Atlas,light:&SkyLight,eye:DVec3,time:f32,solid:&mut ChunkMesh,layers:&mut ChunkMesh) {
    for p in scene.banner_positions_near(eye) {
        let at=DVec3::new(p.0 as f64+0.5,p.1 as f64,p.2 as f64+0.5);
        if at.distance_squared(eye)>64.0*64.0{continue;}
        let Some(b)=Scene::block(scene,p) else{continue};
        let angle=if b.id.path.ends_with("_wall_banner") {
            match b.properties.get("facing").map(String::as_str){Some("south")=>0.0,Some("west")=>90.0,Some("north")=>180.0,_=>270.0}
        }else {b.properties.get("rotation").and_then(|s|s.parse::<f32>().ok()).unwrap_or(0.0)*22.5};
        let pose=Mat4::from_translation(at.as_vec3())*Mat4::from_rotation_y(-angle.to_radians())*Mat4::from_scale(Vec3::new(1.0,-1.0,-1.0)/24.0);
        let phase=((p.0 as f64*7.0+p.1 as f64*9.0+p.2 as f64*13.0+time as f64*20.0).rem_euclid(100.0)/100.0) as f32;
        append(b,scene.banner_tag(p),atlas,[light.get(p) as f32,light.get_block(p) as f32],pose,phase,solid,layers);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::World;
    #[test]
    fn banner_special_model_uses_native_hand_and_ground_scales() {
        let packs=crate::pack::PackStack::open(vec![std::path::PathBuf::from(std::env::var_os("MC_PACK").expect("native pack"))]).unwrap();
        let id=ResourceId::parse("minecraft:white_banner").unwrap();
        for (context,scale) in [("thirdperson_righthand",0.375),("firstperson_righthand",0.375),("ground",0.25)] {
            let pose=crate::model::item_display_transform(&packs,&id,context).unwrap();
            for axis in [Vec3::X,Vec3::Y,Vec3::Z] {assert!((pose.transform_vector3(axis).length()-scale).abs()<1e-6,"{context}");}
        }
    }
    #[test]
    fn banner_is_selectable_without_collision_and_drops_when_support_breaks() {
        let mut scene=HandcraftedScene::default();let p=(0,1,0);
        scene.set((0,0,0),Some(Block::new("minecraft:stone")));
        scene.set(p,Some(Block::new("minecraft:white_banner").with("rotation","0")));
        let mut stack=ItemStack::new("minecraft:white_banner",1);
        stack.components=Some(serde_json::json!({"minecraft:banner_patterns":[{"pattern":"minecraft:creeper","color":"blue"}]}));
        scene.placed_item(p,&stack);
        let mut player=minecraftoss_player::Player::new(DVec3::new(0.5,-0.12,3.5));
        player.yaw=180.0;player.pitch=0.0;
        assert_eq!(player.target(&scene,5.0).unwrap().pos,p);
        let block=World::block(&scene,p).unwrap();
        assert!(minecraftoss_player::authored_collision_boxes(&block).is_empty());
        assert!(remove_unsupported(&mut scene,&[(0,0,0)]).is_empty());
        scene.set((0,0,0),None);
        let dropped=remove_unsupported(&mut scene,&[(0,0,0)]);
        assert_eq!(dropped.len(),1);assert_eq!(dropped[0].1.components,stack.components);
        assert!(Scene::block(&scene,p).is_none());
        scene.set((2,1,0),Some(Block::new("minecraft:stone")));
        scene.set((2,1,1),Some(Block::new("minecraft:yellow_wall_banner").with("facing","south")));
        scene.placed_item((2,1,1),&ItemStack::new("minecraft:yellow_banner",1));
        assert!(remove_unsupported(&mut scene,&[(2,1,0)]).is_empty());
        scene.set((2,1,0),None);
        assert_eq!(remove_unsupported(&mut scene,&[(2,1,0)])[0].1.id,"minecraft:yellow_banner");
    }
    #[test]
    fn banner_placement_break_and_wall_variant_preserve_layers_and_name() {
        let mut scene=HandcraftedScene::default();let p=(-17,64,31);
        let mut stack=ItemStack::new("minecraft:white_banner",1);
        stack.components=Some(serde_json::json!({"minecraft:banner_patterns":[{"pattern":"minecraft:creeper","color":"blue"}],"minecraft:custom_name":"Bandera"}));
        scene.set(p,Some(Block::new("minecraft:white_wall_banner").with("facing","north")));
        scene.placed_item(p,&stack);
        let returned=dropped_stack(Scene::block(&scene,p).unwrap(),scene.banner_tag(p));
        assert_eq!(returned.id,stack.id);assert_eq!(returned.components,stack.components);assert_eq!(returned.max,16);
        assert_eq!(scene.banner_positions().len(),1);
        scene.set(p,None);
        assert!(scene.banner_tag(p).is_none());assert!(scene.banner_positions().is_empty());
        assert!(scene.banner_edit(p).unwrap().is_none());
    }
}

/// The same native cloth and components when carried or dropped.
pub fn item_mesh(stack:&ItemStack,pose:Mat4,at:Vec3,atlas:&Atlas,light:&SkyLight)->ChunkMesh {
    let b=Block::new(&stack.id);let tag=placed_tag((0,0,0),stack);
    let p=(at.x.floor() as i32,at.y.floor() as i32,at.z.floor() as i32);
    let pose=pose*Mat4::from_translation(Vec3::new(0.5,0.0,0.5))*Mat4::from_scale(Vec3::new(1.0,-1.0,-1.0)/24.0);
    let mut solid=ChunkMesh::default();let mut layers=ChunkMesh::default();
    append(&b,Some(&tag),atlas,[light.get(p) as f32,light.get_block(p) as f32],pose,0.0,&mut solid,&mut layers);
    let base=solid.vertices.len() as u32;solid.vertices.extend(layers.vertices);solid.indices.extend(layers.indices.into_iter().map(|n|n+base));solid
}

/// Block.updateShape: removing the floor or wall releases the complete banner.
pub fn remove_unsupported(scene:&mut HandcraftedScene,changes:&[BlockPos])->Vec<(BlockPos,ItemStack)> {
    use minecraftoss_player::World;
    let mut candidates=std::collections::BTreeSet::new();
    for &(x,y,z) in changes {for (dx,dy,dz) in [(0,0,0),(0,1,0),(1,0,0),(-1,0,0),(0,0,1),(0,0,-1)] {candidates.insert((x+dx,y+dy,z+dz));}}
    let mut removed=Vec::new();
    for p in candidates {
        let Some(b)=Scene::block(scene,p).filter(|b|is_banner(b)) else {continue};
        let native=minecraftoss_player::Block{id:b.id.key(),properties:b.properties.clone()};
        if !scene.solid(minecraftoss_player::banner_support(p,&native)) {
            removed.push((p,dropped_stack(b,scene.banner_tag(p))));scene.set(p,None);
        }
    }
    removed
}
