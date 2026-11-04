//! Shared body shapes for portable native geometry. Rasterize only once per
//! source piece, then merge occupied patches; never fill a whole piece's bounds.
//! A patch is at most 1/16 block wide. Its bounds come from the clipped surface,
//! so a horizontal floor keeps its actual height rather than a grid height.
const GRID: i32 = 16;
const THICKNESS: f32 = 1.0 / 4096.0;

fn clip(poly: &[[f32; 3]], axis: usize, plane: f32, above: bool) -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let (da, db) = (a[axis] - plane, b[axis] - plane);
        if if above { da >= 0.0 } else { da <= 0.0 } { out.push(a); }
        if (da < 0.0 && db > 0.0) || (da > 0.0 && db < 0.0) {
            let t = da / (da - db);
            let mut p = std::array::from_fn(|k| a[k] + (b[k] - a[k]) * t);
            p[axis] = plane;
            out.push(p);
        }
    }
    out
}

type Triangle=[[f32;3];3];
type Patch=Option<[f32;6]>;

fn valid(t:&Triangle)->bool {
    if t.iter().flatten().any(|v|!v.is_finite()){return false;}
    let [a,b,c]=t.map(glam::Vec3::from_array);
    !((b-a).cross(c-a).length_squared()<=1e-16)
}

/// Same Y/Z/X order as the old 16³ array, with empty exterior rows omitted.
/// The one-cell guard on varying axes covers interpolation rounding at grid
/// boundaries. Constant planes never move along their constant axis.
struct Patches { first:[i32;3],size:[usize;3],cells:Vec<Patch> }
impl Patches {
    fn bounds(tris:impl Iterator<Item=Triangle>)->Option<([i32;3],[usize;3])> {
        let(mut lo,mut hi)=([f32::INFINITY;3],[f32::NEG_INFINITY;3]);
        for t in tris.filter(valid){for p in t{for k in 0..3{lo[k]=lo[k].min(p[k]);hi[k]=hi[k].max(p[k]);}}}
        if lo[0]==f32::INFINITY{return None;}
        let first=std::array::from_fn(|k|{
            if lo[k]<0.0||hi[k]>1.0{return 0;}
            let n=((lo[k]*GRID as f32).floor()as i32).clamp(0,GRID-1);
            if lo[k]==hi[k]{n}else{(n-1).max(0)}
        });
        let size=std::array::from_fn(|k|{
            if lo[k]<0.0||hi[k]>1.0{return GRID as usize;}
            let n=((hi[k]*GRID as f32).floor()as i32).clamp(0,GRID-1);
            let last=if lo[k]==hi[k]{n}else{(n+1).min(GRID-1)};
            (last-first[k]+1)as usize
        });
        Some((first,size))
    }
    fn new(tris:impl Iterator<Item=Triangle>)->Option<Self> {
        let(first,size)=Self::bounds(tris)?;
        Some(Self{first,size,cells:vec![None;size.iter().product()]})
    }
    fn at(&mut self,cell:[i32;3])->&mut Patch {
        let local=std::array::from_fn::<_,3,_>(|k|(cell[k]-self.first[k])as usize);
        debug_assert!((0..3).all(|k|local[k]<self.size[k]),"clipped cell outside conservative patch bounds");
        &mut self.cells[(local[1]*self.size[2]+local[2])*self.size[0]+local[0]]
    }
}

fn raster(poly: &[[f32; 3]], axis: usize, cell: &mut [i32; 3], patches: &mut Patches) {
    if poly.len() < 3 { return; }
    if axis == 3 {
        let origin = glam::Vec3::from_array(poly[0]);
        let area = (1..poly.len()-1).map(|i|(glam::Vec3::from_array(poly[i])-origin).cross(glam::Vec3::from_array(poly[i+1])-origin)).sum::<glam::Vec3>();
        if area.length_squared() <= 1e-18 { return; }
        // A plane on a grid boundary can be emitted twice. The adjacent cell
        // receives identical bounds; final merging removes this duplication.
        let mut b = [f32::MAX; 6];
        for p in poly { for k in 0..3 { b[k] = b[k].min(p[k]); b[k + 3] = if b[k + 3] == f32::MAX { p[k] } else { b[k + 3].max(p[k]) }; } }
        let patch=patches.at(*cell);
        if let Some(old) = patch {
            for k in 0..3 { old[k] = old[k].min(b[k]); old[k + 3] = old[k + 3].max(b[k + 3]); }
        } else { *patch = Some(b); }
        return;
    }
    let lo = poly.iter().map(|p| p[axis]).fold(f32::INFINITY, f32::min);
    let hi = poly.iter().map(|p| p[axis]).fold(f32::NEG_INFINITY, f32::max);
    let first = ((lo * GRID as f32).floor() as i32).clamp(0, GRID - 1);
    let last = ((hi * GRID as f32).floor() as i32).clamp(0, GRID - 1);
    if first==last && poly.iter().all(|p|p[axis]>=first as f32/GRID as f32 && p[axis]<=(first+1)as f32/GRID as f32) {
        // Both old clips only copied these vertices, in the same order.
        cell[axis]=first;raster(poly,axis+1,cell,patches);return;
    }
    for n in first..=last {
        let part = clip(poly, axis, n as f32 / GRID as f32, true);
        let part = clip(&part, axis, (n + 1) as f32 / GRID as f32, false);
        cell[axis] = n;
        raster(&part, axis + 1, cell, patches);
    }
}

fn merge(boxes: &mut Vec<[f32; 6]>, axis: usize) {
    let other=[[1,2,4,5],[0,2,3,5],[0,1,3,4]][axis];
    boxes.sort_unstable_by(|a, b| {
        for &k in &other { let cmp = a[k].total_cmp(&b[k]); if !cmp.is_eq() { return cmp; } }
        a[axis].total_cmp(&b[axis]).then_with(|| a[axis + 3].total_cmp(&b[axis + 3]))
    });
    let mut write = 0;
    for read in 0..boxes.len() {
        let b = boxes[read];
        if write > 0 && other.iter().all(|&k| boxes[write - 1][k] == b[k]) && b[axis] <= boxes[write - 1][axis + 3] {
            boxes[write - 1][axis + 3] = boxes[write - 1][axis + 3].max(b[axis + 3]);
        } else { boxes[write] = b; write += 1; }
    }
    boxes.truncate(write);
}

pub(crate) fn boxes<I>(tris:I)->Vec<[f32;6]>
where I:IntoIterator<Item=Triangle>, I::IntoIter:Clone {
    let tris=tris.into_iter();
    let Some(mut patches)=Patches::new(tris.clone())else{return Vec::new();};
    for t in tris.filter(valid){raster(&t,0,&mut[0;3],&mut patches);}
    let mut out:Vec<_>=patches.cells.into_iter().flatten().collect();
    // Merge before thickening, so flat floors and walls usually become just
    // one rectangle. Repeating axes joins rows produced by the previous pass.
    for _ in 0..2 { for axis in [0, 2, 1] { merge(&mut out, axis); } }
    for b in &mut out {
        for k in 0..3 {
            if b[k + 3] - b[k] < THICKNESS {
                // Never raise a floor. Walls remain hittable from both sides.
                if k == 1 { b[k] = (b[k + 3] - THICKNESS).max(0.0); }
                else { let mid = (b[k] + b[k + 3]) * 0.5; b[k] = (mid - THICKNESS * 0.5).max(0.0); b[k + 3] = (mid + THICKNESS * 0.5).min(1.0); }
                if b[k] == b[k + 3] { b[k + 3] = (b[k] + THICKNESS).min(1.0); }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    mod reference {
//! Shared body shapes for portable native geometry. Rasterize only once per
//! source piece, then merge occupied patches; never fill a whole piece's bounds.
//! A patch is at most 1/16 block wide. Its bounds come from the clipped surface,
//! so a horizontal floor keeps its actual height rather than a grid height.
const GRID: i32 = 16;
const THICKNESS: f32 = 1.0 / 4096.0;

fn clip(poly: &[[f32; 3]], axis: usize, plane: f32, above: bool) -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let (da, db) = (a[axis] - plane, b[axis] - plane);
        if if above { da >= 0.0 } else { da <= 0.0 } { out.push(a); }
        if (da < 0.0 && db > 0.0) || (da > 0.0 && db < 0.0) {
            let t = da / (da - db);
            let mut p = std::array::from_fn(|k| a[k] + (b[k] - a[k]) * t);
            p[axis] = plane;
            out.push(p);
        }
    }
    out
}

fn raster(poly: &[[f32; 3]], axis: usize, cell: &mut [i32; 3], patches: &mut [Option<[f32; 6]>]) {
    if poly.len() < 3 { return; }
    if axis == 3 {
        let origin = glam::Vec3::from_array(poly[0]);
        let area = (1..poly.len()-1).map(|i|(glam::Vec3::from_array(poly[i])-origin).cross(glam::Vec3::from_array(poly[i+1])-origin)).sum::<glam::Vec3>();
        if area.length_squared() <= 1e-18 { return; }
        // A plane on a grid boundary can be emitted twice. The adjacent cell
        // receives identical bounds; final merging removes this duplication.
        let mut b = [f32::MAX; 6];
        for p in poly { for k in 0..3 { b[k] = b[k].min(p[k]); b[k + 3] = if b[k + 3] == f32::MAX { p[k] } else { b[k + 3].max(p[k]) }; } }
        let index = ((cell[1] * GRID + cell[2]) * GRID + cell[0]) as usize;
        if let Some(old) = &mut patches[index] {
            for k in 0..3 { old[k] = old[k].min(b[k]); old[k + 3] = old[k + 3].max(b[k + 3]); }
        } else { patches[index] = Some(b); }
        return;
    }
    let lo = poly.iter().map(|p| p[axis]).fold(f32::INFINITY, f32::min);
    let hi = poly.iter().map(|p| p[axis]).fold(f32::NEG_INFINITY, f32::max);
    let first = ((lo * GRID as f32).floor() as i32).clamp(0, GRID - 1);
    let last = ((hi * GRID as f32).floor() as i32).clamp(0, GRID - 1);
    for n in first..=last {
        let part = clip(poly, axis, n as f32 / GRID as f32, true);
        let part = clip(&part, axis, (n + 1) as f32 / GRID as f32, false);
        cell[axis] = n;
        raster(&part, axis + 1, cell, patches);
    }
}

fn merge(boxes: &mut Vec<[f32; 6]>, axis: usize) {
    let other: Vec<_> = (0..6).filter(|&k| k != axis && k != axis + 3).collect();
    boxes.sort_unstable_by(|a, b| {
        for &k in &other { let cmp = a[k].total_cmp(&b[k]); if !cmp.is_eq() { return cmp; } }
        a[axis].total_cmp(&b[axis]).then_with(|| a[axis + 3].total_cmp(&b[axis + 3]))
    });
    let mut write = 0;
    for read in 0..boxes.len() {
        let b = boxes[read];
        if write > 0 && other.iter().all(|&k| boxes[write - 1][k] == b[k]) && b[axis] <= boxes[write - 1][axis + 3] {
            boxes[write - 1][axis + 3] = boxes[write - 1][axis + 3].max(b[axis + 3]);
        } else { boxes[write] = b; write += 1; }
    }
    boxes.truncate(write);
}

pub(super) fn boxes(tris: impl IntoIterator<Item = [[f32; 3]; 3]>) -> Vec<[f32; 6]> {
    // Fixed bound regardless of how many original faces occupy this piece.
    let mut patches = vec![None; (GRID * GRID * GRID) as usize];
    for t in tris {
        if t.iter().flatten().any(|v| !v.is_finite()) { continue; }
        let [a, b, c] = t.map(glam::Vec3::from_array);
        if (b - a).cross(c - a).length_squared() <= 1e-16 { continue; }
        raster(&t, 0, &mut [0; 3], &mut patches);
    }
    let mut out: Vec<_> = patches.into_iter().flatten().collect();
    // Merge before thickening, so flat floors and walls usually become just
    // one rectangle. Repeating axes joins rows produced by the previous pass.
    for _ in 0..2 { for axis in [0, 2, 1] { merge(&mut out, axis); } }
    for b in &mut out {
        for k in 0..3 {
            if b[k + 3] - b[k] < THICKNESS {
                // Never raise a floor. Walls remain hittable from both sides.
                if k == 1 { b[k] = (b[k + 3] - THICKNESS).max(0.0); }
                else { let mid = (b[k] + b[k + 3]) * 0.5; b[k] = (mid - THICKNESS * 0.5).max(0.0); b[k + 3] = (mid + THICKNESS * 0.5).min(1.0); }
                if b[k] == b[k + 3] { b[k + 3] = (b[k] + THICKNESS).min(1.0); }
            }
        }
    }
    out
}


    }
    fn assert_exact(tris:&[Triangle],label:&str) {
        let old=reference::boxes(tris.iter().copied());let new=boxes(tris.iter().copied());
        assert_eq!(new.iter().map(|b|b.map(f32::to_bits)).collect::<Vec<_>>(),old.iter().map(|b|b.map(f32::to_bits)).collect::<Vec<_>>(),"{label}: box order and every float bit must match the frozen implementation");
    }
    #[test]
    fn compact_raster_is_bit_exact_for_planes_slopes_gaps_and_boundary_noise() {
        for y in [-0.0,0.0,1.0/4096.0,0.37,0.5,1.0] {
            let floor=rectangle(0.0,1.0,y);assert_exact(&floor,"floor");
            for axis in [0,2] {let wall=floor.map(|t|t.map(|mut p|{p.swap(axis,1);p}));assert_exact(&wall,"wall");}
        }
        let fixtures=vec![
            vec![[[0.0,0.0,0.0],[1.0,1.0,0.0],[1.0,1.0,1.0]],[[0.0,0.0,0.0],[1.0,1.0,1.0],[0.0,0.0,1.0]]],
            rectangle(0.0,0.125,0.37).into_iter().chain(rectangle(0.875,1.0,0.37)).collect(),
            vec![[[-0.0001,0.0,-0.0001],[1.0001,0.5,0.3],[0.4,1.0001,1.0001]]],
            vec![[[f32::MAX,0.0,0.0],[-f32::MAX,1.0,0.0],[0.0,0.0,1.0]]],
            vec![[[0.0;3];3],[[f32::NAN,0.0,0.0],[1.0,0.0,0.0],[0.0,0.0,1.0]],[[f32::INFINITY;3];3]],
            vec![[[0.0625-f32::EPSILON,0.4,0.125],[0.0625+f32::EPSILON,0.6,0.9375],[0.0625,0.50000006,0.5]]],
        ];
        for (n,triangles) in fixtures.iter().enumerate(){assert_exact(triangles,&format!("fixture {n}"));}
        let mut rng=0x7261_7374_6572_3735u64;
        let mut value=||{rng^=rng<<13;rng^=rng>>7;rng^=rng<<17;(rng>>40)as f32/(1u32<<24)as f32};
        for n in 0..48 {
            let tris:Vec<Triangle>=(0..1+n%5).map(|_|std::array::from_fn(|_|std::array::from_fn(|_|value()*1.00002-0.00001))).collect();
            assert_exact(&tris,&format!("deterministic triangle mix {n}"));
        }
        let (_,size)=Patches::bounds(rectangle(0.0,1.0,0.37).into_iter()).unwrap();
        assert_eq!(size,[16,1,16]);
        assert_eq!(size.iter().product::<usize>(),256,"a flat floor no longer initializes all4096 slots");
    }

    fn native_fixtures()->Vec<(String,Vec<Triangle>)> {
        use std::collections::{BTreeMap,BTreeSet};
        let mut columns=vec![(5,crate::jak_pieces::column(crate::jak_pieces::TOWER_SOURCE,(3000,3000)).expect("source5 original tower fixture"))];
        let mut tiles=BTreeSet::from([(4128,4064),(4048,4064)]);
        let ramp=crate::jak_skate_tower::actors().iter().find(|a|a.kind=="skate-training-ramp").expect("original skatea ramp actor");
        let first=ramp.source_cells.first().expect("ramp has visible source cells");
        tiles.insert((first.0.div_euclid(16)*16,first.2.div_euclid(16)*16));
        for (x,z) in tiles {columns.push((7,crate::jak_skate_tower::collision_column(x,z).expect("source7 real collision fixture")));}
        let mut fixtures=Vec::new();let mut kinds_seen=0u8;
        for (source,column) in columns {
            let rows:BTreeMap<_,_>=column.pieces.iter().filter(|(_,p)|!p.tris.is_empty()).map(|(&c,p)|(c,p)).collect();
            let mut selected:BTreeSet<_>=rows.keys().step_by((rows.len()/24).max(1)).take(24).copied().collect();
            let mut wanted=[None;4];
            for (&cell,piece) in &rows {
                let mut floor=false;let mut wall=false;let mut slope=false;
                for t in &piece.tris {
                    let [a,b,c]=t.p.map(glam::Vec3::from_array);let n=(b-a).cross(c-a);
                    if n.length_squared()<=1e-16 {continue;}
                    floor|=t.p.iter().all(|p|p[1]==t.p[0][1]);
                    wall|=[0,2].into_iter().any(|k|t.p.iter().all(|p|p[k]==t.p[0][k]));
                    slope|=n.y!=0.0&&(n.x!=0.0||n.z!=0.0);
                }
                for (kind,yes) in [floor,wall,slope,piece.tris.len()>=8].into_iter().enumerate() {
                    if yes {wanted[kind].get_or_insert(cell);if source==7{kinds_seen|=1<<kind;}}
                }
            }
            selected.extend(wanted.into_iter().flatten());
            if let Some((&cell,_))=rows.iter().max_by_key(|(_,p)|p.tris.len()) {selected.insert(cell);}
            for cell in selected {let p=rows[&cell];
                fixtures.push((format!("source{source} {cell:?} {} triangles, {} blended",p.tris.len(),p.tris.iter().filter(|t|t.blend).count()),p.tris.iter().map(|t|t.p).collect()));
            }
        }
        assert_eq!(kinds_seen,15,"source7 fixtures must cover a real floor, wall, ramp and multi-face piece");
        fixtures
    }

    #[test]
    fn compact_raster_matches_frozen_boxes_for_original_source5_and_source7_pieces() {
        let fixtures=native_fixtures();assert!(fixtures.iter().any(|(n,_)|n.starts_with("source5")));
        assert!(fixtures.iter().any(|(n,_)|n.starts_with("source7")));
        for (name,tris) in fixtures {assert_exact(&tris,&name);}
    }

    #[test]
    #[ignore="bounded cold old/new collision benchmark; run alone after correctness tests"]
    fn report_native_collision_cold_and_warm_cost() {
        use std::{hint::black_box,time::{Duration,Instant}};
        let fixtures=native_fixtures();
        for (name,tris) in &fixtures {assert_exact(tris,name);}
        let slots:usize=fixtures.iter().filter_map(|(_,t)|Patches::bounds(t.iter().copied())).map(|(_,s)|s.iter().product::<usize>()).sum();
        let mut old=Duration::ZERO;let mut new=Duration::ZERO;let mut count=0usize;
        // Alternate order to avoid consistently granting one path the first pass.
        for round in 0..4 {for current in if round%2==0{[false,true]}else{[true,false]} {
            let start=Instant::now();
            for (_,tris) in &fixtures {
                let made=if current{boxes(black_box(tris).iter().copied())}else{reference::boxes(black_box(tris).iter().copied())};
                count+=black_box(made).len();
            }
            if current {new+=start.elapsed();}else{old+=start.elapsed();}
        }}
        let pieces:Vec<_>=fixtures.iter().map(|(_,tris)|{
            let mut p=crate::jak_pieces::Piece::default();
            p.tris=tris.iter().map(|&p|crate::jak_pieces::PieceTri{p,uv:[[0.0;2];3],color:[[128;4];3],texture:0,blend:false}).collect();
            let _=p.collision_boxes();p
        }).collect();
        let start=Instant::now();for _ in 0..100 {for p in &pieces {black_box(p.collision_boxes());}}
        eprintln!("NATIVE_COLLISION_BENCH pieces={} rounds=4 cold_old_ms={:.3} cold_new_ms={:.3} ratio={:.4} old_patch_slots={} active_patch_slots={} patch_bytes={} warm_reads={} warm_ms={:.3} checksum={count}",fixtures.len(),old.as_secs_f64()*1000.0,new.as_secs_f64()*1000.0,new.as_secs_f64()/old.as_secs_f64(),fixtures.len()*4096,slots,std::mem::size_of::<Patch>(),pieces.len()*100,start.elapsed().as_secs_f64()*1000.0);
    }

    fn rectangle(x0: f32, x1: f32, y: f32) -> [[[f32; 3]; 3]; 2] {
        [[[x0,y,0.0],[x1,y,0.0],[x1,y,1.0]], [[x0,y,0.0],[x1,y,1.0],[x0,y,1.0]]]
    }
    #[test]
    fn disjoint_surfaces_keep_their_gap_and_flat_floors_keep_their_height() {
        let shape = boxes(rectangle(0.0, 0.125, 0.37).into_iter().chain(rectangle(0.875, 1.0, 0.37)));
        assert!(shape.len() <= 4, "flat patches must merge: {shape:?}");
        assert!(shape.iter().all(|b| b[3] <= 0.125 || b[0] >= 0.875), "the central corridor must stay empty");
        assert!(shape.iter().all(|b| b[4] == 0.37));
        assert!(shape.iter().all(|b| b.iter().all(|v| (0.0..=1.0).contains(v))));
    }
    #[test]
    fn sloped_surface_bounds_are_local_instead_of_a_whole_solid_cube() {
        let shape = boxes([[[0.0,0.0,0.0],[1.0,1.0,0.0],[1.0,1.0,1.0]], [[0.0,0.0,0.0],[1.0,1.0,1.0],[0.0,0.0,1.0]]]);
        assert!(shape.len() <= 64);
        assert!(shape.iter().all(|b| b[4] - b[1] <= 1.0 / GRID as f32 + THICKNESS));
        assert!(!shape.iter().any(|b| b[0] < 0.2 && b[4] > 0.8));
    }

    #[test]
    fn a_wide_minecraft_body_passes_the_open_space_between_native_walls() {
        use minecraftoss_player::{Block, World};
        use minecraftoss_player::collision::{move_body, Bounds};
        use glam::DVec3;
        struct TestWorld(Vec<[f32; 6]>);
        impl World for TestWorld {
            fn block(&self, p:(i32,i32,i32))->Option<Block>{(p==(0,0,0)).then(||Block::new("jak:pieza_haven"))}
            fn set_block(&mut self,_:(i32,i32,i32),_:Option<Block>){}
            fn collision_boxes(&self,p:(i32,i32,i32))->Vec<[f64;6]>{if p==(0,0,0){self.0.iter().map(|b|b.map(f64::from)).collect()}else{Vec::new()}}
        }
        // Two perpendicular walls used to share a full-cube bounding box.
        let world = TestWorld(boxes([
            [[0.0,0.0,0.0],[0.0,1.0,0.0],[0.0,1.0,1.0]],
            [[0.0,0.0,0.0],[0.0,1.0,1.0],[0.0,0.0,1.0]],
            [[0.0,0.0,0.0],[1.0,1.0,0.0],[0.0,1.0,0.0]],
            [[0.0,0.0,0.0],[1.0,0.0,0.0],[1.0,1.0,0.0]],
        ]));
        let bounds = Bounds::standing(DVec3::new(0.5,0.05,0.5),0.9,1.4);
        let wanted = DVec3::new(0.0,0.0,1.0);
        assert_eq!(move_body(&world,bounds,wanted,false,0.0),wanted);
        let into_wall = move_body(&world,bounds,DVec3::new(-1.0,0.0,0.0),false,0.0);
        assert!(into_wall.x > -0.051 && into_wall.x < 0.0, "real wall still stops the same body: {into_wall:?}");
    }
}
