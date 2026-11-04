//! The ctywide palace silhouette, adapted to a complete city block.
//! Every metre is an ordinary collectible Jak piece with its native UVs.
//! Only the entrance and lobby are opened for the player's actual body.
use std::collections::HashMap;
use std::sync::{Arc,Mutex,OnceLock};
use glam::Vec3;
use crate::jak_items::{BlockLook,JakBlock,JakItem,JakKind};
use crate::jak_pieces::{Source,Column};
use crate::jak_structures::Placed;

pub const ENTRANCE:&str="jak:jetboard_entrada";
pub const BLOCKS:[JakBlock;22]=[
    JakBlock{path:"jetboard_entrada",look:BlockLook::All("citywide-dmnd-01")},
    JakBlock{path:"hv_citywide-dmnd-01",look:BlockLook::All("citywide-dmnd-01")},
    JakBlock{path:"hv_citywide-palace-01",look:BlockLook::All("citywide-palace-01")},
    JakBlock{path:"hv_citywide-window-litwindow",look:BlockLook::All("citywide-window-litwindow")},
    JakBlock{path:"hv_ctywide-ox-met-01",look:BlockLook::All("ctywide-ox-met-01")},
    JakBlock{path:"hv_t-cab",look:BlockLook::All("t-cab")},
    JakBlock{path:"hv_t-citypal-met-bm-red-strp01",look:BlockLook::All("t-citypal-met-bm-red-strp01")},
    JakBlock{path:"hv_t-citypal-met-pill-01",look:BlockLook::All("t-citypal-met-pill-01")},
    JakBlock{path:"hv_t-citypal-met-strp01",look:BlockLook::All("t-citypal-met-strp01")},
    JakBlock{path:"hv_t-citypal-red-met-01",look:BlockLook::All("t-citypal-red-met-01")},
    JakBlock{path:"hv_t-citywide-met-bm-red-strp01",look:BlockLook::All("t-citywide-met-bm-red-strp01")},
    JakBlock{path:"hv_t-citywide-met-pill-01",look:BlockLook::All("t-citywide-met-pill-01")},
    JakBlock{path:"hv_t-citywide-met-strp01",look:BlockLook::All("t-citywide-met-strp01")},
    JakBlock{path:"hv_t-citywide-met-wall-02",look:BlockLook::All("t-citywide-met-wall-02")},
    JakBlock{path:"hv_t-citywide-red-met-01",look:BlockLook::All("t-citywide-red-met-01")},
    JakBlock{path:"hv_t-citywide-wall-tile-01",look:BlockLook::All("t-citywide-wall-tile-01")},
    JakBlock{path:"hv_t-palshaft-dirt-blue-01",look:BlockLook::All("t-palshaft-dirt-blue-01")},
    JakBlock{path:"hv_t-palshaft-panl-01",look:BlockLook::All("t-palshaft-panl-01")},
    JakBlock{path:"hv_t-palshaft-pil-01",look:BlockLook::All("t-palshaft-pil-01")},
    JakBlock{path:"hv_t-palshaft-plate01",look:BlockLook::All("t-palshaft-plate01")},
    JakBlock{path:"hv_t-palshaft-r-strp-plate01",look:BlockLook::All("t-palshaft-r-strp-plate01")},
    JakBlock{path:"hv_t-palshaft-roof-01",look:BlockLook::All("t-palshaft-roof-01")},
];
pub const ITEMS:[JakItem;22]=[
    JakItem{path:"jetboard_entrada",title:"Acceso al circuito de jetboard",kind:JakKind::Block},
    JakItem{path:"hv_citywide-dmnd-01",title:"Baldosa diamantada de Haven",kind:JakKind::Block},
    JakItem{path:"hv_citywide-palace-01",title:"Revestimiento del palacio",kind:JakKind::Block},
    JakItem{path:"hv_citywide-window-litwindow",title:"Ventana iluminada del palacio",kind:JakKind::Block},
    JakItem{path:"hv_ctywide-ox-met-01",title:"Metal oxidado de Haven",kind:JakKind::Block},
    JakItem{path:"hv_t-cab",title:"Panel de cabina",kind:JakKind::Block},
    JakItem{path:"hv_t-citypal-met-bm-red-strp01",title:"Viga roja del palacio",kind:JakKind::Block},
    JakItem{path:"hv_t-citypal-met-pill-01",title:"Pilar de metal del palacio",kind:JakKind::Block},
    JakItem{path:"hv_t-citypal-met-strp01",title:"Banda de metal del palacio",kind:JakKind::Block},
    JakItem{path:"hv_t-citypal-red-met-01",title:"Placa roja del palacio",kind:JakKind::Block},
    JakItem{path:"hv_t-citywide-met-bm-red-strp01",title:"Viga roja de Haven",kind:JakKind::Block},
    JakItem{path:"hv_t-citywide-met-pill-01",title:"Pilar de metal de Haven",kind:JakKind::Block},
    JakItem{path:"hv_t-citywide-met-strp01",title:"Banda de metal de Haven",kind:JakKind::Block},
    JakItem{path:"hv_t-citywide-met-wall-02",title:"Muro de metal de Haven",kind:JakKind::Block},
    JakItem{path:"hv_t-citywide-red-met-01",title:"Metal rojo de Haven",kind:JakKind::Block},
    JakItem{path:"hv_t-citywide-wall-tile-01",title:"Baldosa del palacio de Haven",kind:JakKind::Block},
    JakItem{path:"hv_t-palshaft-dirt-blue-01",title:"Panel azul envejecido",kind:JakKind::Block},
    JakItem{path:"hv_t-palshaft-panl-01",title:"Panel del fuste de la torre",kind:JakKind::Block},
    JakItem{path:"hv_t-palshaft-pil-01",title:"Pilar del fuste de la torre",kind:JakKind::Block},
    JakItem{path:"hv_t-palshaft-plate01",title:"Placa del palacio de Haven",kind:JakKind::Block},
    JakItem{path:"hv_t-palshaft-r-strp-plate01",title:"Placa rayada de la torre",kind:JakKind::Block},
    JakItem{path:"hv_t-palshaft-roof-01",title:"Cubierta del palacio",kind:JakKind::Block},
];
pub const SIZE:i32=22;
pub const HEIGHT:i32=40;
const LEGACY_HEIGHT:i32=32;
const CANVAS:i32=28;
const EXPANDED_AT:i32=3000;
pub const PIECE:&str="jak:pieza_haven_torre";
// Reserved source coordinates outside Haven's three slums. Item components
// already preserve source coordinates when a piece is mined and replaced.
const AT:i32=2000;
pub(crate) fn source_column(place:usize,x:i32,z:i32)->bool {
    place==1 && x<AT+SIZE && x+16>AT && z<AT+SIZE && z+16>AT
}
pub(crate) fn source_cell(place:usize,p:(i32,i32,i32))->bool {
    place==1&&(AT..AT+SIZE).contains(&p.0)&&(AT..AT+SIZE).contains(&p.2)
        || place==crate::jak_pieces::TOWER_SOURCE&&(EXPANDED_AT..EXPANDED_AT+CANVAS).contains(&p.0)&&(EXPANDED_AT..EXPANDED_AT+CANVAS).contains(&p.2)
}
pub(crate) fn source()->Option<&'static Source>{
    static SOURCE:OnceLock<Option<Source>>=OnceLock::new();
    SOURCE.get_or_init(||load_source(AT,SIZE,0.05,Vec3::new(192.0,26.247,600.0))).as_ref()
}
pub(crate) fn source_expanded()->Option<&'static Source>{
    static SOURCE:OnceLock<Option<Source>>=OnceLock::new();
    SOURCE.get_or_init(||load_source(EXPANDED_AT,CANVAS,0.0625,Vec3::new(189.56,26.247,599.72))).as_ref()
}
fn load_source(at:i32,canvas:i32,scale:f32,pivot:Vec3)->Option<Source>{
        let root=crate::original_places::levels_root("jak2")?;
        let names=crate::spargus_city_blocks::HAVEN_SLUMS_TEXTURES.iter().enumerate().map(|(i,t)|(t.texture,i as u16)).collect();
        let mut tris=Vec::new();
        crate::original_places::read_level_detailed(&root.join("ctywide/ctywide-background.glb"),&names,
            [10.0,382.0,410.0,780.0,25.0,670.0],&mut tris).ok()?;
        // ctywide exports this distant landmark as BLEND even where both
        // vertex and texture alpha are fully opaque (128 is PS2 opacity).
        // Those faces must write depth as a building here, otherwise the
        // later cloud pass draws through its roof. Keep any real fades.
        let mut opaque=HashMap::new();
        for t in &tris {opaque.entry(t.texture).or_insert_with(||{
            let name=crate::spargus_city_blocks::HAVEN_SLUMS_TEXTURES[t.texture as usize].texture;
            crate::original_places::embedded_texture(name).is_some_and(|(_,_,rgba)|rgba.chunks_exact(4).all(|p|p[3]>=128))
        });}
        for t in &mut tris {
            if opaque[&t.texture] && t.color.iter().all(|c|c[3]>=128){t.blend=false;}
            t.p=t.p.map(|p|(p-pivot)*scale+Vec3::new((at+canvas/2) as f32,0.0,(at+canvas/2) as f32));
        }
        (!tris.is_empty()).then(||Source::new(tris))
}
fn cells(expanded:bool)->&'static Option<Arc<Column>>{
    static LEGACY:OnceLock<Option<Arc<Column>>>=OnceLock::new();
    static EXPANDED:OnceLock<Option<Arc<Column>>>=OnceLock::new();
    let (slot,at,size)=if expanded {(&EXPANDED,EXPANDED_AT,CANVAS)} else {(&LEGACY,AT,SIZE)};
    slot.get_or_init(||{
        let started=std::time::Instant::now();
        let source=if expanded {source_expanded()?} else {source()?};
        let column=crate::jak_pieces::cut_area(source,(at,at+size-1),(at,at+size-1));
        // Generation/prefetch pays this once. Drawing and every actor body
        // subsequently share the same source pieces and their cached shapes.
        for piece in column.values(){let _=piece.collision_boxes();}
        let mut counts:Vec<_>=column.values().map(|p|p.collision_boxes().len()).collect();counts.sort_unstable();
        let total:usize=counts.iter().sum();
        if !counts.is_empty(){
            diag::info!(World,"tower collision v{}: {} pieces, median {} p95 {} max {} boxes, {} bytes in {:.3}s",if expanded{2}else{1},counts.len(),counts[counts.len()/2],counts[counts.len()*95/100],counts.last().unwrap(),total*std::mem::size_of::<[f32;6]>(),started.elapsed().as_secs_f64());
            #[cfg(test)] eprintln!("tower collision v{} cold {:.3}s: median {} p95 {} max {} total {} boxes / {} bytes",if expanded{2}else{1},started.elapsed().as_secs_f64(),counts[counts.len()/2],counts[counts.len()*95/100],counts.last().unwrap(),total,total*std::mem::size_of::<[f32;6]>());
        }
        Some(Arc::new(column))
    })
}
fn pieces()->Option<&'static Column>{cells(false).as_deref()}
fn expanded_pieces()->Option<&'static Column>{
    cells(true).as_deref()
}
pub(crate) fn column(expanded:bool)->Option<Arc<Column>>{cells(expanded).clone()}
fn opening(x:i32,y:i32,z:i32)->bool {
    (0..4).contains(&y) && ((10..=12).contains(&x)&&(11..SIZE).contains(&z)
        || (7..=14).contains(&x)&&(7..=14).contains(&z))
}
type Origin=(i32,i32,i32);
type Towers=HashMap<(i64,i32,i32),Vec<Origin>>;
static TOWERS:OnceLock<Mutex<Towers>>=OnceLock::new();
// Generation choice is used only for diagnostics/copying a freshly built
// district. Saved block IDs always select their own source independently.
static GENERATED_VERSION:OnceLock<Mutex<HashMap<(i64,Origin),bool>>>=OnceLock::new();
fn generated_expanded(seed:i64,origin:Origin)->bool{GENERATED_VERSION.get().and_then(|v|v.lock().ok()).and_then(|v|v.get(&(seed,origin)).copied()).unwrap_or(false)}
pub fn diagnostic(seed:i64,feet:[f64;3])->Vec<serde_json::Value>{
    let Some(t)=TOWERS.get().and_then(|t|t.lock().ok()).map(|t|t.clone())else{return Vec::new();};
    let mut seen=std::collections::HashSet::new();let mut out=Vec::new();
    for ((s,_,_),origins) in t.iter(){if *s!=seed{continue;}for &o in origins {
        let c=[o.0+11,o.1,o.2+11];
        if !seen.insert(o)||(c[0] as f64-feet[0]).abs()>180.0||(c[2] as f64-feet[2]).abs()>180.0{continue;}
        if !GENERATED_VERSION.get().and_then(|v|v.lock().ok()).is_some_and(|v|v.contains_key(&(seed,o))){continue;}
        let expanded=generated_expanded(seed,o);
        let (source,at,margin,height)=if expanded{(crate::jak_pieces::TOWER_SOURCE,EXPANDED_AT,3,HEIGHT)}else{(1,AT,0,LEGACY_HEIGHT)};
        let sample=(if expanded{expanded_pieces()}else{pieces()}).and_then(|ps|ps.keys().find(|p|p.1>height-8)).copied();
        let mesh_triangles=sample.and_then(|p|crate::jak_pieces::piece_at(source,(o.0+p.0-at-margin,o.1+p.1,o.2+p.2-at-margin)))
            .and_then(|(_,column,cell)|column.get(&cell).map(|p|p.tris.len()));
        out.push(serde_json::json!({"center":c,"height":height,"source_version":if expanded{2}else{1},"source_seed":crate::original_places::current_world().map(|w|w.0),"sample_triangles":mesh_triangles,
            "exterior":[c[0] as f64+0.5,c[1] as f64+0.05,c[2] as f64+14.0],
            "door":[c[0] as f64+0.5,c[1] as f64+0.05,c[2] as f64+10.4],
            "threshold":[c[0] as f64+0.5,c[1] as f64+0.05,c[2] as f64-1.5]}));
    }}out
}
fn remember(seed:i64,origin:Origin){
    let mut towns=TOWERS.get_or_init(Default::default).lock().unwrap_or_else(|e|e.into_inner());
    for x in (origin.0-3).div_euclid(16)..=(origin.0+SIZE+2).div_euclid(16){for z in (origin.2-3).div_euclid(16)..=(origin.2+SIZE+2).div_euclid(16){
        let entries=towns.entry((seed,x,z)).or_default();if !entries.contains(&origin){entries.push(origin);}
    }}
}
pub(crate) fn source_at(pos:Origin)->Option<(usize,Origin)>{
    let seed=crate::original_places::current_world()?.0;
    source_at_seed(seed,pos)
}
pub(crate) fn source_at_seed(seed:i64,pos:Origin)->Option<(usize,Origin)>{
    let towns=TOWERS.get()?.lock().ok()?;
    for &o in towns.get(&(seed,pos.0.div_euclid(16),pos.2.div_euclid(16)))? {
        let (x,y,z)=(pos.0-o.0,pos.1-o.1,pos.2-o.2);
        if (0..SIZE).contains(&x)&&(0..LEGACY_HEIGHT).contains(&y)&&(0..SIZE).contains(&z)&&!opening(x,y,z){return Some((1,(AT+x,y,AT+z)));}
    }None
}
pub(crate) fn source_at_expanded(pos:Origin)->Option<(usize,Origin)>{
    let seed=crate::original_places::current_world()?.0;
    if let Some(found)=source_at_expanded_seed(seed,pos){return Some(found);}
    // Saved chunks can arrive before any generation worker rebuilt this
    // session's city index. The prop loader reconstructs the district once
    // per missing chunk, including its tower, without modifying saved blocks.
    let _=crate::jak_town::original_at(pos);
    source_at_expanded_seed(seed,pos)
}
pub(crate) fn source_at_expanded_seed(seed:i64,pos:Origin)->Option<(usize,Origin)>{
    let towns=TOWERS.get()?.lock().ok()?;
    for &o in towns.get(&(seed,pos.0.div_euclid(16),pos.2.div_euclid(16)))? {
        let (x,y,z)=(pos.0-o.0,pos.1-o.1,pos.2-o.2);
        if (-3..SIZE+3).contains(&x)&&(0..HEIGHT).contains(&y)&&(-3..SIZE+3).contains(&z)&&!opening(x,y,z){return Some((crate::jak_pieces::TOWER_SOURCE,(EXPANDED_AT+x+3,y,EXPANDED_AT+z+3)));}
    }None
}
/// A flat, street-connected plot. Its landmark remains breakable one
/// block at a time; the portal floor is also an obtainable building item.
pub fn building(seed:i64,center:Origin)->Vec<Placed>{
    building_version(seed,center,true)
}
/// The larger silhouette is all-or-nothing at biome boundaries. Keep the
/// valid old plot and its origin when its high crown would cross the biome.
pub fn building_on(seed:i64,center:Origin,land:&dyn crate::jak_structures::Land)->Vec<Placed>{
    let expanded=expanded_pieces().is_some_and(|pieces|pieces.keys().all(|p| {
        let(x,z)=(center.0+p.0-EXPANDED_AT-CANVAS/2,center.2+p.2-EXPANDED_AT-CANVAS/2);
        land.city_allowed(x,z,false)&&!land.wet(x,z)
    }));
    building_version(seed,center,expanded)
}
fn building_version(seed:i64,center:Origin,expanded:bool)->Vec<Placed>{
    let o=(center.0-11,center.1,center.2-11);remember(seed,o);
    GENERATED_VERSION.get_or_init(Default::default).lock().unwrap_or_else(|e|e.into_inner()).insert((seed,o),expanded);
    building_plan(center,expanded)
}
/// Legacy geometry for replaying old deterministic prop choices. This must
/// not change the generated version or the metadata of the active city.
pub(crate) fn legacy_building(center:Origin)->Vec<Placed>{building_plan(center,false)}
/// New towns keep the former civic lot and all deterministic house/prop
/// identities, while the full physical circuit stands beside the city.
/// Remembering the old source origin lets already-saved tower cells load.
pub(crate) fn civic_plaza(seed:i64,center:Origin)->Vec<Placed>{
    let o=(center.0-11,center.1,center.2-11);remember(seed,o);
    let mut out=Vec::new();
    for x in 0..SIZE {for z in 0..SIZE {
        out.push(((o.0+x,o.1-1,o.2+z),"jak:hv_t-citywide-wall-tile-01"));
        for y in 0..4 {out.push(((o.0+x,o.1+y,o.2+z),"minecraft:air"));}
    }}out
}
fn building_plan(center:Origin,expanded:bool)->Vec<Placed>{
    let Some(pieces)=(if expanded{expanded_pieces()}else{pieces()})else{return Vec::new();};
    let (source_at,margin,height)=if expanded{(EXPANDED_AT,3,HEIGHT)}else{(AT,0,LEGACY_HEIGHT)};
    let o=(center.0-11,center.1,center.2-11);
    let at=|x,y,z|(o.0+x,o.1+y,o.2+z);
    let mut plan=crate::jak_families::Plan::default();
    for x in 0..SIZE {for z in 0..SIZE {
        plan.fixed(at(x,-1,z),"jak:hv_t-citywide-wall-tile-01");
        for y in 0..height {plan.fixed(at(x,y,z),"minecraft:air");}
    }}
    for (&(sx,y,sz),piece) in &pieces.pieces {
        let(x,z)=(sx-source_at-margin,sz-source_at-margin);if !(0..height).contains(&y)||opening(x,y,z){continue;}
        let state=match (expanded,crate::jak_objects::slab_type(piece)){
            (true,"top")=>"jak:pieza_haven_torre[type=top,waterlogged=false]",
            (true,"double")=>"jak:pieza_haven_torre[type=double,waterlogged=false]",
            (true,_)=>"jak:pieza_haven_torre[type=bottom,waterlogged=false]",
            (false,"top")=>"jak:pieza_haven[type=top,waterlogged=false]",
            (false,"double")=>"jak:pieza_haven[type=double,waterlogged=false]",
            (false,_)=>"jak:pieza_haven[type=bottom,waterlogged=false]",
        };plan.fixed(at(x,y,z),state);
    }
    // Frame the cut as a deliberate three-block entrance, with original
    // palace metal, a retracting gate and a clear corridor to the lobby.
    for x in 9..=13 {for y in 0..=3 {
        if x==9||x==13||y==3 {plan.fixed(at(x,y,19),"jak:hv_t-palshaft-plate01");}
    }}
    for x in 10..=12 {plan.fixed(at(x,0,19),"jak:krimson_porton_fence_gate[facing=south,in_wall=false,open=false,powered=false]");}
    for x in 6..=15 {for z in 6..=15 {for y in 0..=4 {
        if y==4 || ((x==6||x==15||z==6||z==15)&&!((10..=12).contains(&x)&&z>=11)) {
            plan.fixed(at(x,y,z),"jak:hv_t-citywide-wall-tile-01");
        }
    }}}
    // Continue to the back wall: a visitor walking into the room must
    // remain on the entrance floor rather than overshoot it by one tile.
    for x in 9..=12 {for z in 7..=10 {plan.fixed(at(x,-1,z),ENTRANCE);}}
    for (x,z) in [(7,7),(14,7),(7,14),(14,14)] {plan.fixed(at(x,3,z),"jak:krimson_luz");}
    for x in [7,14] {
        plan.fixed(at(x,0,12),"jak:krimson_terminal[facing=south]");
        plan.fixed(at(x,0,13),"minecraft:chest[facing=south,type=single,waterlogged=false]");
    }
    plan.finish()
}

pub(crate) fn relocate(seed:i64,center:(i32,i32),delta:Origin){
    let origins= TOWERS.get().and_then(|t|t.lock().ok()).map(|t|t.iter()
        .filter(|((s,_,_),_)|*s==seed).flat_map(|(_,v)|v.iter().copied())
        .filter(|o|(o.0+11-center.0).abs()<=60&&(o.2+11-center.1).abs()<=60)
        .collect::<std::collections::HashSet<_>>()).unwrap_or_default();
    let mut moved=Vec::new();
    for o in origins {
        let expanded=generated_expanded(seed,o);
        let (kind,at,margin,height)=if expanded{(crate::jak_pieces::TOWER_SOURCE,EXPANDED_AT,3,HEIGHT)}else{(1,AT,0,LEGACY_HEIGHT)};
        let Some(cells)=(if expanded{expanded_pieces()}else{pieces()})else{continue;};
        for &cell in cells.keys(){let(x,y,z)=(cell.0-at-margin,cell.1,cell.2-at-margin);
        if (0..height).contains(&y)&&!opening(x,y,z){
            moved.push(((o.0+x+delta.0,o.1+y+delta.1,o.2+z+delta.2),Some((kind,cell))));
        }
    }}
    if let Err(e)=crate::jak_pieces::set_moved_batch(moved){diag::warn!(World,"tower source commit failed: {e}");}
}

#[cfg(test)]mod tests {
    use super::*;
    #[test]fn a_saved_expanded_tower_rebuilds_its_origin_in_a_cold_process(){
        const CHILD:&str="IW4L_TEST_TOWER_COLD_CHILD";
        if std::env::var_os(CHILD).is_none(){
            let output=std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact","jak_tower::tests::a_saved_expanded_tower_rebuilds_its_origin_in_a_cold_process","--nocapture","--test-threads=1"])
                .env(CHILD,"1").output().expect("isolated cold-load test process");
            assert!(output.status.success(),"cold load failed:\n{}\n{}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));
            return;
        }
        assert!(TOWERS.get().is_none(),"this test must start without any tower index");
        let paths=minecraftoss_core::registries::DataPaths::discover().unwrap();
        let registries=Arc::new(minecraftoss_core::Registries::load(&paths).unwrap());
        crate::original_places::bind_world(registries,12345);
        // Fixture is the tower in the saved seed12345 Haven used by the CE
        // real-game run. Loading geometry does not generate its city/index.
        let center=(-2834,70,-5489);
        let (&cell,piece)=expanded_pieces().unwrap().iter().find(|(p,t)|p.1>32&&!t.tris.is_empty()).unwrap();
        let at=(center.0+cell.0-EXPANDED_AT-CANVAS/2,center.1+cell.1,center.2+cell.2-EXPANDED_AT-CANVAS/2);
        assert!(TOWERS.get().is_none());assert!(source_at_expanded_seed(12345,at).is_none());
        let mut scene=crate::scene::HandcraftedScene::default();
        scene.set(at,Some(crate::scene::Block::new(PIECE).with("type","double").with("waterlogged","false")));
        let shape=minecraftoss_player::World::collision_boxes(&scene,at);
        assert!(!shape.is_empty(),"a saved v2 block must recover its geometry before body collision");
        assert_eq!(source_at_expanded_seed(12345,at),Some((crate::jak_pieces::TOWER_SOURCE,cell)));
        assert_eq!(shape,piece.collision_boxes().iter().map(|b|b.map(f64::from)).collect::<Vec<_>>());
        let (_,column,found)=crate::jak_pieces::piece_at(crate::jak_pieces::TOWER_SOURCE,at).unwrap();
        assert_eq!(column[&found].tris.len(),piece.tris.len());
        // The recovered index also keeps the old scale addressable by an
        // actual legacy block ID, rather than substituting the v2 source.
        let legacy=(center.0-10,center.1+5,center.2-10);
        assert_eq!(source_at_seed(12345,legacy),Some((1,(AT+1,5,AT+1))));
    }
    #[test]fn native_tower_collision_is_cached_bounded_and_shared_with_minecraft_bodies(){
        use minecraftoss_player::World;
        for (source_id,id,cells) in [(1,"jak:pieza_haven",pieces().expect("legacy palace source")),(crate::jak_pieces::TOWER_SOURCE,PIECE,expanded_pieces().expect("expanded palace source"))] {
        let mut counts=Vec::new();let mut total=0;let started=std::time::Instant::now();
        let mut gap=None;
        for (&cell,piece) in cells.iter(){
            let shape=piece.collision_boxes();assert!(std::sync::Arc::ptr_eq(&shape,&piece.collision_boxes()));
            assert!(shape.len()<=4096);counts.push(shape.len());total+=shape.len();
            if gap.is_none() && shape.len()>1 {
                let lo: [f32;3]=std::array::from_fn(|k|shape.iter().map(|b|b[k]).fold(1.0,f32::min));
                let hi: [f32;3]=std::array::from_fn(|k|shape.iter().map(|b|b[k+3]).fold(0.0,f32::max));
                for x in 1..16 {for y in 1..16 {for z in 1..16 {
                    let p=[x as f32/16.0,y as f32/16.0,z as f32/16.0];
                    if (0..3).all(|k|p[k]>lo[k]+0.02&&p[k]<hi[k]-0.02)
                        && !shape.iter().any(|b|(0..3).all(|k|p[k]+0.015>b[k]&&p[k]-0.015<b[k+3])) {gap=Some((cell,p));break;}
                }}}
            }
        }
        counts.sort_unstable();
        eprintln!("native tower collision source{source_id}: {} pieces, median {}, p95 {}, max {}, total {} boxes/{} bytes, warm {:.3}s",counts.len(),counts[counts.len()/2],counts[counts.len()*95/100],counts.last().unwrap(),total,total*std::mem::size_of::<[f32;6]>(),started.elapsed().as_secs_f64());
        let (cell,p)=gap.expect("a real palace piece has space inside its enclosing box");
        let at=(32123+source_id as i32,80,32123);crate::jak_pieces::set_moved(at,Some((source_id,cell)));
        let mut scene=crate::scene::HandcraftedScene::default();
        scene.set(at,Some(crate::scene::Block::new(id).with("type","double").with("waterlogged","false")));
        let minecraft=World::collision_boxes(&scene,at);
        let shared=crate::jak_pieces::collision_boxes(source_id as u8,[at.0,at.1,at.2]).unwrap();
        assert_eq!(minecraft,shared.iter().map(|b|b.map(f64::from)).collect::<Vec<_>>());
        assert!(!minecraft.iter().any(|b|(0..3).all(|k|f64::from(p[k])+0.015>b[k]&&f64::from(p[k])-0.015<b[k+3])));
        scene.set(at,None);assert!(World::collision_boxes(&scene,at).is_empty(),"mining removes collision even while its source is cached");
        crate::jak_pieces::set_moved(at,None);
        }
    }
    #[test]fn opaque_native_tower_faces_write_depth_in_front_of_clouds(){
        let s=source().expect("ctywide original export");assert!(s.tris.len()>5000);
        assert!(s.tris.iter().all(|t|!t.blend),"this source's palace textures and vertex alpha are opaque");
        let cells=pieces().unwrap();
        assert!(cells.values().flat_map(|p|&p.tris).all(|t|!t.blend),"cutting into portable blocks keeps depth writes");
    }
    #[test]fn native_palace_keeps_its_silhouette_and_every_piece_is_portable(){
        let s=source().expect("ctywide original export");assert!(s.tris.len()>5000);
        let cells=pieces().unwrap();assert!(cells.len()>800);
        let top=cells.keys().map(|p|p.1).max().unwrap();assert!(top>=31);
        let (&cell,expected)=cells.iter().find(|(p,_)|p.1>24).unwrap();
        let component=crate::jak_pieces::item_components(1,cell);
        assert_eq!(crate::jak_pieces::carried(Some(&component)),Some((1,cell)));
        let moved=(32001,80,32001);crate::jak_pieces::set_moved(moved,Some((1,cell)));
        let (_,column,found)=crate::jak_pieces::piece_at(1,moved).unwrap();
        assert_eq!(column[&found].tris.len(),expected.tris.len());
        crate::jak_pieces::set_moved(moved,None);
        for name in ["t-palshaft-plate01","citywide-palace-01","citywide-dmnd-01"]{
            assert!(crate::original_places::embedded_texture(name).is_some(),"native texture {name}");
        }
    }
    #[test]fn expanded_tower_preserves_legacy_source_and_keeps_street_level_in_its_plot(){
        let before=source().unwrap().tris[0].p;
        let s=source_expanded().unwrap();assert!(s.tris.len()>5000);
        assert_eq!(source().unwrap().tris[0].p,before,"loading v2 must not rescale old source coordinates");
        let center=(EXPANDED_AT+CANVAS/2) as f32;
        for p in s.tris.iter().flat_map(|t|t.p){
            assert!(p.x>=EXPANDED_AT as f32&&p.x<=(EXPANDED_AT+CANVAS) as f32);
            assert!(p.z>=EXPANDED_AT as f32&&p.z<=(EXPANDED_AT+CANVAS) as f32);
            assert!(p.y<HEIGHT as f32);
            if p.y<4.0 {assert!((p.x-center).abs()<11.0&&(p.z-center).abs()<11.0,"street-level geometry must fit the unchanged plot");}
        }
        let seed=77331;let base=(45000,70,45000);remember(seed,base);
        let at=(45001,75,45001);
        assert_eq!(source_at_seed(seed,at),Some((1,(AT+1,5,AT+1))));
        assert_eq!(source_at_expanded_seed(seed,at),Some((crate::jak_pieces::TOWER_SOURCE,(EXPANDED_AT+4,5,EXPANDED_AT+4))));
        assert!(source_at_seed(seed,(45001,105,45001)).is_none());
        assert!(source_at_expanded_seed(seed,(45001,105,45001)).is_some());
        for (kind,cells) in [(1,pieces().unwrap()),(crate::jak_pieces::TOWER_SOURCE,expanded_pieces().unwrap())] {
            let (&cell,piece)=cells.iter().find(|(p,_)|p.1>=if kind==1 {24}else{33}).unwrap();
            let json=serde_json::to_string(&crate::jak_pieces::item_components(kind,cell)).unwrap();
            let value=serde_json::from_str(&json).unwrap();
            let carried=crate::jak_pieces::carried(Some(&value)).unwrap();assert_eq!(carried,(kind,cell));
            let at=(45200+kind as i32,80,45200);crate::jak_pieces::set_moved(at,Some(carried));
            let (source,column,found)=crate::jak_pieces::piece_at(kind,at).unwrap();
            assert_eq!(source,kind);assert_eq!(found,cell);assert_eq!(column[&found].tris.len(),piece.tris.len());
            assert!(Arc::ptr_eq(&column[&found].collision_boxes(),&piece.collision_boxes()));
            crate::jak_pieces::set_moved(at,None);
        }
    }
    #[test]fn a_biome_edge_uses_the_whole_legacy_tower_instead_of_cutting_the_new_crown(){
        struct Edge;impl crate::jak_structures::Land for Edge {
            fn ground(&self,_:i32,_:i32)->i32{70}fn wet(&self,_:i32,_:i32)->bool{false}
            fn city_allowed(&self,_x:i32,z:i32,spargus:bool)->bool{!spargus&&z>=-13}
        }
        let made=building_on(77332,(0,70,0),&Edge);
        assert!(made.iter().filter(|(_,s)|s.starts_with("jak:pieza_haven[")).count()>800);
        assert!(!made.iter().any(|(_,s)|s.starts_with(PIECE)));
        assert!(made.iter().all(|(p,_)|p.2>=-13));
        assert!(source_at_seed(77332,(-10,75,-10)).is_some(),"the legacy source origin remains registered");
    }
    #[test]fn the_lobby_is_reachable_from_an_operable_three_panel_entrance(){
        let made:HashMap<_,_>=building(12345,(11,70,11)).into_iter().collect();
        let registered:std::collections::HashSet<_>=crate::jak_items::block_looks().map(|b|b.path).collect();
        let paths=minecraftoss_core::registries::DataPaths::discover().unwrap();
        let registry=minecraftoss_core::Registries::load(&paths).unwrap();
        for state in made.values().filter_map(|s|s.strip_prefix("jak:")) {
            assert!(registered.contains(state.split('[').next().unwrap()),"unregistered tower block {state}");
            registry.blocks.parse_state(&format!("jak:{state}")).expect(state);
        }
        crate::original_places::bind_world(std::sync::Arc::new(registry),12345);
        let (&cell,expected)=expanded_pieces().unwrap().iter().find(|(p,_)|p.1>32).unwrap();
        let at=(cell.0-EXPANDED_AT-3,70+cell.1,cell.2-EXPANDED_AT-3);
        assert!(made[&at].starts_with(PIECE),"new geometry must be selected by a saved v2 block ID");
        let (_,column,found)=crate::jak_pieces::piece_at(crate::jak_pieces::TOWER_SOURCE,at).expect("generated v2 tower must reach the mesher above legacy height");
        assert_eq!(column[&found].tris.len(),expected.tris.len());
        assert_eq!(made.values().filter(|s|s.starts_with("jak:krimson_porton")).count(),3);
        for x in 10..=12 {for z in 11..22 {for y in 70..73 {
            let b=made[&(x,y,z)];assert!(b=="minecraft:air"||z==19&&y==70&&b.starts_with("jak:krimson_porton"),"{x} {y} {z}: {b}");
        }}}
        assert_eq!(made[&(10,69,9)],ENTRANCE);
        assert!(made.values().any(|s|s.starts_with("minecraft:chest")));
    }
    #[test]fn the_city_keeps_its_civic_plot_and_precinct_without_a_portal_tower(){
        struct Flat;impl crate::jak_structures::Land for Flat{fn ground(&self,_:i32,_:i32)->i32{70}fn wet(&self,_:i32,_:i32)->bool{false}}
        for turn in 0..4 {let made=crate::jak_barrio::village(12345,(100,200),70,turn,&Flat);
            assert_eq!(made.iter().filter(|(_,s)|*s==ENTRANCE).count(),0,"the physical circuit must not create a portal in rotation {turn}");
            assert_eq!(made.iter().filter(|(_,s)|s.starts_with(crate::jak_krimson::POST)).count(),2,"keep the guard precinct");
            assert!(!made.iter().any(|(_,s)|s.starts_with(PIECE)),"old miniature tower must not overlap the new civic square");
            assert!(made.iter().filter(|(_,s)|*s=="jak:hv_t-citywide-wall-tile-01").count()>=SIZE as usize*SIZE as usize,"keep the complete paved civic lot");
        }
        let spargus=crate::jak_spargus::town(12345,(100,200),70,0,&Flat);
        assert!(!spargus.iter().any(|(_,s)|*s==ENTRANCE));
    }
}
