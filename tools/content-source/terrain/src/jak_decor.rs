//! Complete native Hip Hog compositions, reusable as grid-aligned decoration.
use glam::Vec3;
use crate::jak_pieces::SourceTri;
pub const TEXTURES:&[&str]=&["hip-tbotblue02","hip-tgreendark01","krew-belt","hip-tbluelit01","hip-grunt-metal-01","hip-grunt-hose","hip-grunt-gem-01","hip-grunt-skin-03","hip-grunt-eye-01","hip-snacker-orange","hip-bab-eye","hip-snacker-tan","hip-snacker-mouth","hip-snacker-gold","hip-snacker-skin-02","hip-snacker-tooth","hip-snacker-skin-01","hip-grunt-skin-01","hip-grunt-teeth-01","common-white","hip-tmetring02","hip-twood02","hip-tmetfloor11","hip-tmetfloor01","hip-tboothlight01","hip-wac-02","hip-tmetbooth01","hipwac-screens","hip-wac-01","hip-wac-03","hip-tamblit01","hip-tmetring01"];
pub const LEVELS:&[(&str,&str)]=&[("lhipout","hip-bottle-c-lod0.glb"),("lhipout","hip-mug-lod0.glb"),("hiphog","hip-trophy-a-lod0.glb"),("hiphog","hip-trophy-c-lod0.glb"),("hiphog","hip-trophy-f-lod0.glb"),("hiphog","hip-whack-a-metal-lod0.glb"),("hiphog","hip-mirror-lod0.glb")];
pub fn contains(block:&str)->bool{ matches!(block,"objeto_haven_servicio_bar"|"objeto_haven_botellero_bar"|"objeto_haven_mesa_doble_bar"|"objeto_haven_trofeo_grunt_bar"|"objeto_haven_trofeo_pez_bar"|"objeto_haven_trofeo_metal_bar"|"objeto_haven_juego_bar"|"objeto_haven_espejo_bar"|"objeto_haven_coleccion_bar"|"objeto_haven_mostrador_bar")}
pub fn read(block:&str)->Option<Vec<SourceTri>>{
 let parts:&[(usize,[f32;3],[f32;3],f32)]=match block {
"objeto_haven_servicio_bar"=>&[(0,[0.24, 0.5, 0.24],[0.3, 0.06, 0.5],0.0),(1,[0.24, 0.27, 0.24],[0.67, 0.06, 0.3],0.0),(1,[0.24, 0.27, 0.24],[0.67, 0.06, 0.7],2.0)],
"objeto_haven_botellero_bar"=>&[(0,[0.23, 0.55, 0.23],[0.22, 0.06, 0.5],0.0),(0,[0.23, 0.55, 0.23],[0.5, 0.06, 0.5],0.0),(0,[0.23, 0.55, 0.23],[0.78, 0.06, 0.5],0.0)],
"objeto_haven_mesa_doble_bar"=>&[(1,[0.3, 0.32, 0.3],[0.3, 0.06, 0.5],0.0),(1,[0.3, 0.32, 0.3],[0.7, 0.06, 0.5],2.0)],
"objeto_haven_trofeo_grunt_bar"=>&[(2,[0.8, 0.85, 0.8],[0.5, 0.06, 0.5],0.0)],
"objeto_haven_trofeo_pez_bar"=>&[(3,[0.8, 0.85, 0.8],[0.5, 0.06, 0.5],0.0)],
"objeto_haven_trofeo_metal_bar"=>&[(4,[0.8, 0.85, 0.8],[0.5, 0.06, 0.5],0.0)],
"objeto_haven_juego_bar"=>&[(5,[0.9, 0.94, 0.9],[0.5, 0.0, 0.5],0.0)],
"objeto_haven_espejo_bar"=>&[(6,[0.85, 0.9, 0.3],[0.5, 0.0, 0.5],0.0)],
"objeto_haven_coleccion_bar"=>&[(2,[0.36, 0.5, 0.36],[0.27, 0.06, 0.5],0.0),(3,[0.36, 0.5, 0.36],[0.73, 0.06, 0.5],0.0)],
"objeto_haven_mostrador_bar"=>&[(0,[0.23, 0.55, 0.23],[0.22, 0.06, 0.4],0.0),(0,[0.23, 0.45, 0.23],[0.5, 0.06, 0.35],0.0),(1,[0.25, 0.28, 0.25],[0.75, 0.06, 0.65],1.0)],
 _=>return None,};
 let place=crate::original_places::named("haven")?;
 let root=crate::original_places::levels_root("jak2")?;
 let names=crate::original_places::textures_of(place).iter().enumerate().map(|(i,&n)|(n,i as u16)).collect();
 let mut out=Vec::new();
 for &(asset,size,center,turn) in parts {
  let (folder,file)=LEVELS[asset];let mut tris=Vec::new();
  crate::original_places::read_level_detailed(&root.join(folder).join(file),&names,[-10000.,10000.,-10000.,10000.,-10000.,10000.],&mut tris).map_err(|e|{eprintln!("decoration {file}: {e}");e}).ok()?;
  let lo=tris.iter().flat_map(|t|t.p).reduce(Vec3::min)?;let hi=tris.iter().flat_map(|t|t.p).reduce(Vec3::max)?;
  let extent=hi-lo;let scale=(Vec3::from_array(size)/extent.max(Vec3::splat(0.001))).min_element();
  let pivot=Vec3::new((lo.x+hi.x)*0.5,lo.y,(lo.z+hi.z)*0.5);
  let rotation=glam::Quat::from_rotation_y(turn*std::f32::consts::FRAC_PI_2);
  for t in &mut tris{t.p=t.p.map(|p|rotation*((p-pivot)*scale)+Vec3::from_array(center));}out.extend(tris);
  if matches!(asset,2|3|4) {
   let mut stand=Vec::new();crate::jak_bar::tray(&mut stand,*names.get("hip-tmetring01")?);
   // A visible metal stem connects the irregular native trophy to its plinth.
   for t in &mut stand {t.p=t.p.map(|p|Vec3::new(center[0]+(p.x-0.5)*0.16,0.06+p.y*5.,center[2]+(p.z-0.5)*0.16));}out.extend(stand);
  }

 }
 if parts.iter().all(|p|p.2[1]>0.){crate::jak_bar::tray(&mut out,*names.get("hip-tmetring01")?);}
 Some(out)
}

#[cfg(test)] mod tests {
 #[test] fn decor33_native_compositions_have_complete_materials_and_fit_one_cell() {
  for kind in 95..crate::jak_objects_catalog::OBJECTS.len() {
   let o=&crate::jak_objects_catalog::OBJECTS[kind];
   let tris=super::read(o.block).expect(o.block);
   assert!(tris.len()>12,"{} has no complete geometry",o.block);
   for t in &tris {for p in t.p {assert!(p.is_finite()&&p.min_element()>=-0.001&&p.max_element()<=1.001,"{}: {p:?}",o.block);}}
   let shape=crate::jak_objects::shape(kind).expect(o.block);
   assert_eq!(shape.size,(1,1,1),"{}",o.block);
   assert!(!shape.cells[&(0,0,0)].collision_boxes().is_empty(),"{}",o.block);
  }
  let (_,_,glass)=crate::original_places::embedded_texture("hip-tbotblue02").unwrap();
  assert!(glass.chunks_exact(4).any(|p|p[3]==128),"native masked bottle must survive the cutout pass");
  for &name in super::TEXTURES {assert!(crate::original_places::embedded_texture(name).is_some(),"missing {name}");}
 }
}
