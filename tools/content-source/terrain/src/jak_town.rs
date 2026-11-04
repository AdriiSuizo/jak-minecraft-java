//! Urban districts shared by Villa Refugio and Spargus. Four by four
//! neighbourhoods, small squares, shops and a network of through streets.
use crate::jak_families::Site;
use crate::jak_structures::{Land, Placed};
pub const MARK: &str = "minecraft:lodestone";
pub const EXPANDED_MARK: &str = "minecraft:black_glazed_terracotta";
pub const SPARGUS_MARK: &str = "minecraft:yellow_glazed_terracotta";
pub const ROUTE_MARK: &str = "minecraft:light_blue_glazed_terracotta";
pub const CITY_MARK: &str = "minecraft:cyan_glazed_terracotta";
pub const PLAZA: i32 = 18;
pub const RING_OUT: i32 = 23;
pub const FRONT: i32 = 26;
pub const DEPTH: i32 = 12;
pub const REACH: i32 = 104;
pub const STREETS: [i32; 7] = [-84, -56, -28, 0, 28, 56, 84];

/// Keep cities on broad, gentle sites. A flat plaza alone must not allow
/// a city to slice a neighbouring mountain or build a tall shoreline wall.
pub(crate) fn gentle_site(land:&dyn Land,cx:i32,cz:i32,y:i32)->bool {
    let mut lo=y;let mut hi=y;
    for dx in [-56,-28,0,28,56] {for dz in [-56,-28,0,28,56] {
        let (h,wet)=land.probe_column(cx+dx,cz+dz);if wet{continue;}lo=lo.min(h);hi=hi.max(h);
        if (h-y).abs()>12 || hi-lo>16{return false;}
    }}true
}

#[derive(Clone, Copy, Debug)]
pub struct Lot {
    pub origin: (i32, i32, i32),
    pub front: u8,
    pub w: i32,
    pub d: i32,
    pub corner: bool,
}
#[derive(Clone, Debug)]
pub struct Town {
    pub center: (i32, i32, i32),
    pub turn: u8,
    pub ring: Vec<(i32, i32)>,
    pub lots: Vec<Lot>,
    pub station: Option<(i32,i32)>,
    pub tower: Option<(i32,i32)>,
    pub palace: Option<(i32,i32,i32)>,
    heights: Vec<i32>,
    spacious: bool,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ground {
    Plaza,
    Road,
    Street,
    Lot,
}

impl Town {
    /// The palace belongs to the district, never to an unrelated parcel.
    /// Admit it only with real, supported building plots on all four sides.
    pub(crate) fn reserve_palace(&mut self,seed:i64,land:&dyn Land)->bool {
        let c=self.center;
        if c.1<64 || c.1+crate::jak_city_arena::HEIGHT>320 {return false;}
        // High horns may overhang the sea, but not another game's biome.
        for x in (crate::jak_city_arena::FOOTPRINT_MIN[0]..crate::jak_city_arena::FOOTPRINT_MAX[0]) {
            for z in (crate::jak_city_arena::FOOTPRINT_MIN[1]..crate::jak_city_arena::FOOTPRINT_MAX[1]) {
                if !land.city_allowed(c.0+x,c.2+z,false){return false;}
            }
        }
        let mut ring=Vec::new();
        for (axis,sign,front) in [(0,-1,1),(0,1,3),(1,-1,2),(1,1,0)] {
            let start=ring.len();
            for lateral in [-24,0,24] {
                // South's middle is the stair approach: leave it open.
                if axis==1 && sign==1 && lateral==0 {continue;}
                let (x,z)=if axis==0{(sign*76,lateral)}else{(lateral,sign*76)};
                let lot=Lot{origin:(c.0+x,c.1,c.2+z),front,w:12,d:12,corner:false};
                let site=Site{origin:lot.origin,front,w:lot.w,d:lot.d};
                if !(-3..15).all(|x|(-3..15).all(|z|{
                    let p=site.at(x,0,z);let h=land.ground(p.0,p.2);
                    land.city_allowed(p.0,p.2,false)&&(h-c.1).abs()<=24
                })) {continue;}
                ring.push(lot);
            }
            if ring.len()==start {return false;}
        }
        let overlaps=|a:&Lot,b:&Lot|{
            let aw=if a.front%2==0{a.w}else{a.d};let ad=if a.front%2==0{a.d}else{a.w};
            let bw=if b.front%2==0{b.w}else{b.d};let bd=if b.front%2==0{b.d}else{b.w};
            (a.origin.0-b.origin.0).abs()*2<aw+bw+8&&(a.origin.2-b.origin.2).abs()*2<ad+bd+8
        };
        self.lots.retain(|l|{
            let site=Site{origin:l.origin,front:l.front,w:l.w,d:l.d};
            !(-3..l.w+3).any(|x|(-3..l.d+3).any(|z|{
                let p=site.at(x,0,z);Self::palace_reserved(c,p.0,p.2)
            })) && !ring.iter().any(|b|overlaps(l,b))
        });
        // One of the surrounding frontages remains the functioning KG base.
        let station=(seed as u64%ring.len() as u64)as usize;
        self.station=Some((ring[station].origin.0,ring[station].origin.2));
        self.tower=None;self.lots.extend(ring);self.palace=Some(c);
        true
    }
    fn palace_reserved(c:(i32,i32,i32),x:i32,z:i32)->bool {
        let(x,z)=(x-c.0,z-c.2);
        // Reserve the low buttresses and the complete stair/side approach.
        (x+5).pow(2)+(z+5).pow(2)<58*58
            ||((-14..=4).contains(&x)&&(36..=68).contains(&z))
            ||((0..=70).contains(&x)&&(-16..=-4).contains(&z))
    }
    pub(crate) fn palace_owns(&self,p:(i32,i32,i32))->bool {
        self.palace.is_some_and(|c|p.1>=c.1&&p.1<c.1+crate::jak_city_arena::HEIGHT
            &&Self::palace_reserved(c,p.0,p.2))
    }
    /// One whole district for the native palace. Keep the through streets
    /// and the precinct, and reject a plot crossing the allowed biome.
    pub fn reserve_tower(&mut self,seed:i64,land:&dyn Land){
        let sites=[(-42,-42),(42,42),(-42,42),(42,-42)];
        for i in 0..4 {
            let (x,z)=sites[(i+(seed as u64%4) as usize)%4];let p=self.site().at(x,0,z);
            if !(-13..=13).all(|dx|(-13..=13).all(|dz|land.city_allowed(p.0+dx,p.2+dz,false)&&!land.wet(p.0+dx,p.2+dz))){continue;}
            let overlaps=|l:&Lot|{let s=Site{origin:l.origin,front:l.front,w:l.w,d:l.d};
                (0..l.w).any(|x|(0..l.d).any(|z|{let q=s.at(x,0,z);(p.0-11..p.0+11).contains(&q.0)&&(p.2-11..p.2+11).contains(&q.2)}))};
            if self.lots.iter().any(|l|self.station==Some((l.origin.0,l.origin.2))&&overlaps(l)){continue;}
            self.lots.retain(|l|!overlaps(l));
            self.lots.push(Lot{origin:p,front:0,w:22,d:22,corner:false});self.tower=Some((p.0,p.2));return;
        }
    }
    /// Reserve a civic plot after attached frontages have settled, without
    /// overlapping a rotated neighbour or crossing a biome boundary.
    pub fn reserve_station(&mut self,seed:i64,land:&dyn Land){
        let site=self.site();let mut candidates=[(-35,-21,0),(35,21,2)];
        if seed&1!=0{candidates.reverse();}
        let mut occupied=std::collections::HashSet::new();
        for l in &self.lots {let s=Site{origin:l.origin,front:l.front,w:l.w,d:l.d};
            for x in 0..l.w{for z in 0..l.d{let p=s.at(x,0,z);occupied.insert((p.0,p.2));}}
        }
        for (x,z,way) in candidates {
            let at=site.at(x,0,z);let s=Site{origin:at,front:site.way(way),w:11,d:10};
            if !(-2..13).all(|x|(-3..12).all(|z|{let p=s.at(x,0,z);land.city_allowed(p.0,p.2,false)&&!land.wet(p.0,p.2)})) {continue;}
            if (0..11).any(|x|(0..10).any(|z|{let p=s.at(x,0,z);occupied.contains(&(p.0,p.2))})){continue;}
            self.lots.push(Lot{origin:at,front:s.front,w:11,d:10,corner:false});
            self.station=Some((at.0,at.2));break;
        }
    }
    fn local(&self, at:(i32,i32,i32))->(i32,i32){
        let (fx,fz)=crate::jak_facades::dir_step(self.turn);
        let (rx,rz)=crate::jak_facades::dir_step(self.turn+1);
        let (dx,dz)=(at.0-self.center.0,at.2-self.center.2);
        (dx*rx+dz*rz,-dx*fx-dz*fz)
    }
    /// Ctysluma/b's attached frontages: groups share party walls, with
    /// recesses, side streets and differently oriented blocks of houses.
    pub fn expand_haven(&mut self,seed:i64){
        let site=self.site();
        for ix in 0i32..12 {for iz in 0i32..12 {let(x,z)=(ix*14-77,iz*14-77);
            if x.abs()<=49 && z.abs()<=49 {continue;}
        let h=(seed as u64).wrapping_add(ix as u64*7919+iz as u64*104729);
            if h%7==0{continue;}
            self.lots.push(Lot{origin:site.at(x,0,z),front:site.way(if iz%2==0{0}else{2}),w:7+(h%4)as i32,d:7+((h>>4)%3)as i32,corner:ix==0||ix==11||iz==0||iz==11});
        }}
    }
    /// Spargus is a desert settlement: compact buildings around distinct squares,
    /// with broad vehicle streets and room for yards rather than party walls.
    pub fn spread_spargus(&mut self, seed: i64) {
        self.spacious = true;
        self.lots.clear();
        let site = self.site();
        for x in [-42i32, -14, 14, 42] {
            for z in [-70i32, -42, -14, 14, 42, 70] {
                if x.abs() == 14 && z.abs() == 14 || (x, z) == (-14, -42) || (x, z) == (14, 42) { continue; }
                let h = (seed as u64) ^ (x as u64).wrapping_mul(7919) ^ (z as u64).wrapping_mul(104729);
                let corner = x.abs() == 42 && z.abs() == 70;
                // Modest offsets break the grid while preserving nine-block vehicle streets.
                let dx = 0;
                let dz = 0;
                self.lots.push(Lot {
                    origin: site.at(x + dx, 0, z + dz),
                    front: site.way(if z < 0 { 2 } else { 0 }),
                    w: if corner { 12 } else { 16 + (h % 2) as i32 },
                    d: if corner { 12 } else { 14 + ((h >> 4) % 2) as i32 },
                    corner,
                });
            }
        }
    }
    pub fn haven_frontages(&mut self,seed:i64){
        let mut rows=std::collections::BTreeMap::<(i32,i32),Vec<usize>>::new();
        for (i,lot) in self.lots.iter().enumerate(){let (x,z)=self.local(lot.origin);rows.entry(((x+56).div_euclid(28),z)).or_default().push(i);}
        let site=self.site();
        for ((district,z),mut ids) in rows {
            if ids.len()!=2{continue;}
            ids.sort_by_key(|i|self.local(self.lots[*i].origin).0);
            let h=(seed as u64)^(district as u64).wrapping_mul(104729)^((z.div_euclid(28)+2) as u64).wrapping_mul(7919);
            if h%5==0{continue;}
            let left=district*28-53;let widths=[9+(h%5) as i32,13-(h%5) as i32];
            let mut x=left;
            for (part,i) in ids.into_iter().enumerate(){
                let lot=&mut self.lots[i];lot.w=widths[part];
                let recess=if part==1 && h%3==0{1}else{0};
                let zz=z+if z.rem_euclid(28)<14{recess}else{-recess};
                lot.origin=site.at(x+lot.w/2,0,zz);x+=lot.w;
            }
        }
        // Some complete blocks turn a quarter around their own courtyard.
        // The surrounding traffic streets stay continuous in every seed.
        let positions:Vec<_>=self.lots.iter().map(|l|self.local(l.origin)).collect();
        for (lot,(x,z)) in self.lots.iter_mut().zip(positions){
            let (dx,dz)=((x+56).div_euclid(28),(z+56).div_euclid(28));
            let h=(seed as u64)^(dx as u64).wrapping_mul(104729)^(dz as u64).wrapping_mul(7919);
            if h%4==1 {let(cx,cz)=(dx*28-42,dz*28-42);lot.origin=site.at(cx-(z-cz),0,cz+(x-cx));lot.front=(lot.front+1)%4;}
        }
    }
    pub fn site(&self) -> Site {
        Site {
            origin: self.center,
            front: self.turn,
            w: 0,
            d: 0,
        }
    }
    pub fn ground(&self, x: i32, z: i32) -> Ground {
        if self.spacious && [(-14, -42), (14, 42)].iter().any(|&(px,pz)| (x-px).abs()<=10 && (z-pz).abs()<=10) {
            return Ground::Plaza;
        }
        if x.abs() <= if self.spacious { 22 } else { 10 } && z.abs() <= if self.spacious { 22 } else { 10 } {
            Ground::Plaza
        } else if STREETS
            .iter()
            .any(|a| (x - a).abs() <= if self.spacious { 4 } else { 2 } || (z - a).abs() <= if self.spacious { 4 } else { 2 })
        {
            Ground::Road
        } else if (x.abs() <= 2 || z.abs() <= 2) {
            Ground::Street
        } else {
            Ground::Lot
        }
    }
    pub(crate) fn coverage(&self, x: i32, z: i32) -> f32 {
        let site = self.site();
        let at = site.at(x, 0, z);
        let d = self
            .lots
            .iter()
            .map(|l| {
                let mut dx = (at.0 - l.origin.0).abs() as f32;
                let mut dz = (at.2 - l.origin.2).abs() as f32;
                if self.spacious {
                    // Broad Spargus houses are separated by vehicle streets.
                    // Measure from their footprint, not their centre: otherwise
                    // every junction looks like an outskirts gap and loses its
                    // graded paving and NPC route links.
                    let (w,d)=if l.front%2==0{(l.w,l.d)}else{(l.d,l.w)};
                    dx=(dx-w as f32*0.5).max(0.0);
                    dz=(dz-d as f32*0.5).max(0.0);
                }
                (dx * dx + dz * dz).sqrt()
            })
            .fold(((x * x + z * z) as f32).sqrt(), f32::min);
        let d = if self.spacious { [(-14, -42), (14, 42)].iter().fold(d, |d, &(px,pz)| d.min((((x-px).pow(2)+(z-pz).pow(2)) as f32).sqrt())) } else { d };
        // The palace and the neighbouring houses share a paved precinct;
        // treating its centre as a point would leave grass gaps at the base.
        let d=if let Some(c)=self.palace {
            let at=site.at(x,0,z);let dx=(at.0-c.0+5)as f32;let dz=(at.2-c.2+5)as f32;
            d.min((dx.hypot(dz)-50.).max(0.))
        }else{d};
        let t = ((d - 10.0) / 16.0).clamp(0.0, 1.0);
        1.0 - t * t * (3.0 - 2.0 * t)
    }
    /// Surface in eighths, shared across chunks, including each entrance pad.
    pub fn eighths(&self, x: i32, z: i32) -> i32 {
        if self.heights.is_empty() {
            self.center.1 * 8
        } else {
            self.heights[((x + REACH) * (2 * REACH + 1) + z + REACH) as usize]
        }
    }
    pub fn grade(&mut self, seed: i64, land: &dyn Land, spargus: bool) {
        let palace=self.palace;
        self.lots.retain(|lot| {
            // Waterfront frontages use the same real stone foundations as
            // the city's shallow-water streets. Deep water was rejected by
            // reserve_palace; ordinary inland lots keep their existing rule.
            let waterfront=palace.is_some_and(|c|{
                let(x,z)=((lot.origin.0-c.0).abs(),(lot.origin.2-c.2).abs());
                x==76&&z<=24||z==76&&x<=24
            });
            let site = Site {
                origin: lot.origin,
                front: lot.front,
                w: lot.w,
                d: lot.d,
            };
            (-2..lot.w + 2).all(|x| {
                (-3..lot.d + 2).all(|z| {
                    let (x, _, z) = site.at(x, 0, z);
                    land.city_allowed(x,z,spargus) && (waterfront||!land.wet(x,z))
                })
            })
        });
        let site = self.site();
        let y0 = self.center.1 as f32;
        let sample = |x, z| {
            let (x, _, z) = site.at(x, 0, z);
            land.ground(x, z) as f32
        };
        let sx = ((sample(56, 0) - sample(-56, 0)) / 112.0).clamp(-0.045, 0.045);
        let sz = ((sample(0, 56) - sample(0, -56)) / 112.0).clamp(-0.045, 0.045);
        let phase = ((seed as u64 ^ self.center.0 as u64 ^ (self.center.2 as u64).rotate_left(17))
            % 1000) as f32
            * 0.00628;
        let relief = |x: f32, z: f32| {
            let wave = 1.8 * ((x / 31.0 + phase).sin() - phase.sin())
                + 1.5 * ((z / 37.0 - phase).sin() + phase.sin());
            let central = (x.abs().max(z.abs()) / 16.0).clamp(0.0, 1.0);
            y0 + (sx * x + sz * z + wave) * central
        };
        // Each block of four buildings shares a level platform; roads gently
        // connect its neighbours. Never slope a room's own floor.
        let n=if self.lots.iter().any(|l|{let(x,z)=self.local(l.origin);x.abs()>56||z.abs()>56}){6}else{4};
        let mut district = vec![vec![self.center.1;n];n];
        for ix in 0..n {
            for iz in 0..n {
                district[ix][iz] =
                    relief((ix as i32 * 28 - (n as i32-1)*14) as f32, (iz as i32 * 28 - (n as i32-1)*14) as f32).round()
                        as i32;
            }
        }
        for _ in 0..32 {
            for x in 0..n {
                for z in 0..n {
                    for (xx, zz) in [(x + 1, z), (x, z + 1)] {
                        if xx >= n || zz >= n {
                            continue;
                        }
                        let delta = district[x][z] - district[xx][zz];
                        if delta > 1 {
                            if district[x][z] > self.center.1 {
                                district[x][z] -= 1;
                            } else {
                                district[xx][zz] += 1;
                            }
                        } else if delta < -1 {
                            if district[xx][zz] > self.center.1 {
                                district[xx][zz] -= 1;
                            } else {
                                district[x][z] += 1;
                            }
                        }
                    }
                }
            }
        }
        let local_lots: Vec<_> = self
            .lots
            .iter()
            .map(|lot| {
                let pos=self.local(lot.origin);
                (
                    pos.0,
                    pos.1,
                    district[(pos.0.div_euclid(28) + n as i32/2).clamp(0,n as i32-1) as usize]
                        [(pos.1.div_euclid(28) + n as i32/2).clamp(0,n as i32-1) as usize],
                    if (lot.front+self.turn)%2==0{lot.w}else{lot.d},
                    if (lot.front+self.turn)%2==0{lot.d}else{lot.w},
                )
            })
            .collect();
        let mut local_lots = local_lots;
        let building_lots=local_lots.len();
        if let Some(c)=self.palace {
            let (x,z)=self.local(c);local_lots.push((x,z,c.1,110,110));
            for (dx,dz,w,d) in [(-5,60,18,20),(63,-10,20,18)] {
                let(x,z)=self.local((c.0+dx,c.1,c.2+dz));
                let(w,d)=if self.turn%2==0{(w,d)}else{(d,w)};
                local_lots.push((x,z,c.1,w,d));
            }
        }
        if self.spacious {
            for (x,z) in [(-14,-42),(14,42)] {
                local_lots.push((x,z,relief(x as f32,z as f32).round() as i32,18,18));
            }
        }
        for (lot, (_, _, y, _, _)) in self.lots.iter_mut().zip(&local_lots[..building_lots]) {
            lot.origin.1 = *y;
        }
        self.heights.clear();
        let width = (2 * REACH + 1) as usize;
        let mut fixed = Vec::with_capacity(width * width);
        for x in -REACH..=REACH {
            for z in -REACH..=REACH {
                let mut h = relief(x as f32, z as f32);
                let mut sum = h * 0.002;
                let mut weights = 0.002;
                let mut pad = None;
                for &(lx, lz, y, w, d) in &local_lots {
                    let distance = ((x - lx).abs() as f32 - (w / 2 + 1) as f32)
                        .max(0.0)
                        .max(((z - lz).abs() as f32 - (d / 2 + 1) as f32).max(0.0));
                    if distance == 0.0 {
                        pad = Some(y);
                        break;
                    }
                    let weight = 1.0 / (distance + 0.15).powi(4);
                    sum += weight * y as f32;
                    weights += weight;
                }
                h = pad.map_or(sum / weights, |y| y as f32);
                let edge = (1.0 - self.coverage(x, z))
                    .max(((x.abs().max(z.abs()) - if n==6{87}else{59}) as f32 / 17.0).clamp(0.0, 1.0));
                if edge > 0.0 {
                    h += (sample(x, z) - h) * edge * edge * (3.0 - 2.0 * edge);
                }
                let plaza_radius = if self.spacious { 22 } else { 9 };
                let plaza = x.abs() <= plaza_radius && z.abs() <= plaza_radius;
                if plaza {
                    h = y0;
                }
                fixed.push(pad.is_some() || plaza);
                self.heights.push((h * 8.0).round() as i32);
            }
        }
        // The tightest Lipschitz envelopes of all fixed room platforms.
        // Their midpoint preserves every floor and bounds each street step.
        let mut lower: Vec<i32> = self
            .heights
            .iter()
            .enumerate()
            .map(|(i, h)| if fixed[i] { *h } else { -1_000_000 })
            .collect();
        let mut upper: Vec<i32> = self
            .heights
            .iter()
            .enumerate()
            .map(|(i, h)| if fixed[i] { *h } else { 1_000_000 })
            .collect();
        for x in 0..width {
            for z in 0..width {
                let i = x * width + z;
                if x > 0 {
                    lower[i] = lower[i].max(lower[i - width] - 4);
                    upper[i] = upper[i].min(upper[i - width] + 4);
                }
                if z > 0 {
                    lower[i] = lower[i].max(lower[i - 1] - 4);
                    upper[i] = upper[i].min(upper[i - 1] + 4);
                }
            }
        }
        for x in (0..width).rev() {
            for z in (0..width).rev() {
                let i = x * width + z;
                if x + 1 < width {
                    lower[i] = lower[i].max(lower[i + width] - 4);
                    upper[i] = upper[i].min(upper[i + width] + 4);
                }
                if z + 1 < width {
                    lower[i] = lower[i].max(lower[i + 1] - 4);
                    upper[i] = upper[i].min(upper[i + 1] + 4);
                }
            }
        }
        for x in -REACH..=REACH {
            for z in -REACH..=REACH {
                let i = ((x + REACH) * (2 * REACH + 1) + z + REACH) as usize;
                if fixed[i] {
                    continue;
                }
                let smooth = (lower[i] + upper[i]) / 2;
                let edge = (1.0 - self.coverage(x, z))
                    .max(((x.abs().max(z.abs()) - if n==6{87}else{59}) as f32 / 17.0).clamp(0.0, 1.0));
                self.heights[i] = (smooth as f32
                    + (self.heights[i] - smooth) as f32 * edge * edge * (3.0 - 2.0 * edge))
                    .round() as i32;
            }
        }
    }
    pub fn civic(&self, spargus: bool, land: &dyn Land) -> Vec<Placed> {
        let mut out = Vec::new();
        let site = self.site();
        for (x, z) in if spargus { [(-14, -42), (14, 42)] } else { [(-35, -21), (35, 21)] } {
            let at=site.at(x,0,z);
            if !spargus && (self.station==Some((at.0,at.2))||self.palace_owns(at)) {continue;}
            if self.coverage(x, z) < 0.8
                || !(-5..=5).all(|dx| {
                    (-5..=5).all(|dz| {
                        let (wx, _, wz) = site.at(x + dx, 0, z + dz);
                        land.city_allowed(wx, wz, spargus) && !land.wet(wx, wz)
                    })
                })
            {
                continue;
            }
            let y = self.eighths(x, z).div_euclid(8) - self.center.1;
            let floor = if spargus {
                "jak:sparg_arena"
            } else {
                "jak:barrio_losa"
            };
            for dx in -4..=4 {
                for dz in -4..=4 {
                    out.push((site.at(x + dx, y - 1, z + dz), floor));
                }
            }
            for dx in [-3, 3] {
                for dz in [-3, 3] {
                    for h in 0..3 {
                        out.push((
                            site.at(x + dx, y + h, z + dz),
                            if spargus {
                                "jak:spargus_viga[facing=up,powered=false,waterlogged=false]"
                            } else {
                                "jak:barrio_poste[facing=up,powered=false,waterlogged=false]"
                            },
                        ));
                    }
                }
            }
            for dx in -4..=4 {
                for dz in -3..=3 {
                    out.push((site.at(x+dx,y+3,z+dz),if spargus {"jak:spargus_toldo[facing=north,half=top,open=false,powered=false,waterlogged=false]"}else{"jak:barrio_toldo[facing=north]"}));
                }
            }
            for dx in [-2, 2] {
                out.push((
                    site.at(x + dx, y, z + 2),
                    "minecraft:barrel[facing=up,open=false]",
                ));
            }
        }
        for x in [-56, -28, 0, 28, 56] {
            for z in [-56, -28, 0, 28, 56] {
                if x == 0 && z == 0 || self.coverage(x, z) < 0.8 {
                    continue;
                }
                for way in 0..4u8 {
                    let (dx, dz) = crate::jak_facades::dir_step(way);
                    let (nx, nz) = (x + dx * 28, z + dz * 28);
                    if !(-56..=56).contains(&nx)
                        || !(-56..=56).contains(&nz)
                        || (nx == 0 && nz == 0)
                    {
                        continue;
                    }
                    let valid = (0..=28).all(|n| {
                        let (xx, zz) = (x + dx * n, z + dz * n);
                        let (wx, _, wz) = site.at(xx, 0, zz);
                        land.city_allowed(wx, wz, spargus)
                            && !land.wet(wx, wz)
                            && self.coverage(xx, zz) > 0.75
                            && (n == 0
                                || (self.eighths(xx, zz) - self.eighths(xx - dx, zz - dz)).abs()
                                    <= 4)
                    });
                    if valid {
                        out.push((
                            site.at(
                                x,
                                self.eighths(x, z).div_euclid(8)
                                    - self.center.1
                                    - 7
                                    - i32::from(site.way(way)),
                                z,
                            ),
                            site.facing("minecraft:light_blue_glazed_terracotta[facing={f}]", way),
                        ));
                    }
                }
                let verge = if spargus { 5 } else { 3 };
                let (wx, _, wz) = site.at(x + verge, 0, z + verge);
                if !land.city_allowed(wx, wz, spargus) || land.wet(wx, wz) {
                    continue;
                }
                let y = self.eighths(x, z).div_euclid(8) - self.center.1;
                for h in 0..3 {
                    out.push((
                        site.at(x + verge, y + h, z + verge),
                        if spargus {
                            "jak:spargus_viga[facing=up,powered=false,waterlogged=false]"
                        } else {
                            "jak:barrio_poste[facing=up,powered=false,waterlogged=false]"
                        },
                    ));
                }
                out.push((
                    site.at(x + verge, y + 3, z + verge),
                    "minecraft:lantern[hanging=false,waterlogged=false]",
                ));
            }
        }
        out
    }
    /// One layer of authored paving follows the relief. Air is cleared only
    /// through the urban core, and the perimeter meets the original terrain.
    pub fn terrain(&self, land: &dyn Land, spargus: bool) -> Vec<Placed> {
        let site = self.site();
        let mut out = Vec::new();
        for x in -REACH..=REACH {
            for z in -REACH..=REACH {
                let (wx, _, wz) = site.at(x, 0, z);
                let coverage = self.coverage(x, z);
                if coverage < 0.02 || !land.city_allowed(wx, wz, spargus) {
                    continue;
                }
                let wet=land.wet(wx,wz);
                // The protected urban core must bridge shallow ponds too.
                // Skipping wet columns punctured streets beside finished homes.
                // The outer shore keeps its original water and terrain.
                if wet&&coverage<=0.75{continue;}
                let ground = land.ground(wx, wz);
                let eighths = self.eighths(x, z);
                let y = eighths.div_euclid(8);
                let rem = eighths.rem_euclid(8);
                let road = matches!(
                    self.ground(x, z),
                    Ground::Road | Ground::Street | Ground::Plaza
                );
                let whole = if spargus {
                    if road {
                        "jak:sparg_losa"
                    } else {
                        "jak:sparg_arena"
                    }
                } else {
                    "jak:barrio_losa"
                };
                let layers = if spargus {
                    if road { SPARG_ROAD } else { SPARG_SAND }
                } else {
                    HAVEN
                };
                let fill = if wet { whole } else if spargus {
                    "jak:wasteland_rock"
                } else {
                    "minecraft:dirt"
                };
                for yy in ground.min(y - 2)..y - 1 {
                    out.push(((wx, yy, wz), fill));
                }
                let noise = ((wx as i64).wrapping_mul(73_856_093)
                    ^ (wz as i64).wrapping_mul(19_349_663))
                .rem_euclid(100) as f32
                    / 100.0;
                let paving = if coverage >= 0.9 || noise < coverage {
                    whole
                } else {
                    fill
                };
                out.push(((wx, y - 1, wz), paving));
                let core = coverage > 0.75;
                let clear_to = if core {
                    ground.max(y) + 26
                } else {
                    ground.max(y) + 20
                };
                for yy in y..clear_to {
                    out.push(((wx, yy, wz), "minecraft:air"));
                }
                if rem > 0 {
                    out.push(((wx, y, wz), layers[(rem - 1) as usize]));
                }
            }
        }
        out
    }
}
const HAVEN: [&str; 7] = [
    "jak:barrio_losa_capa[layers=1]",
    "jak:barrio_losa_capa[layers=2]",
    "jak:barrio_losa_capa[layers=3]",
    "jak:barrio_losa_capa[layers=4]",
    "jak:barrio_losa_capa[layers=5]",
    "jak:barrio_losa_capa[layers=6]",
    "jak:barrio_losa_capa[layers=7]",
];
const SPARG_ROAD: [&str; 7] = [
    "jak:sparg_losa_capa[layers=1]",
    "jak:sparg_losa_capa[layers=2]",
    "jak:sparg_losa_capa[layers=3]",
    "jak:sparg_losa_capa[layers=4]",
    "jak:sparg_losa_capa[layers=5]",
    "jak:sparg_losa_capa[layers=6]",
    "jak:sparg_losa_capa[layers=7]",
];
const SPARG_SAND: [&str; 7] = [
    "jak:sparg_arena_capa[layers=1]",
    "jak:sparg_arena_capa[layers=2]",
    "jak:sparg_arena_capa[layers=3]",
    "jak:sparg_arena_capa[layers=4]",
    "jak:sparg_arena_capa[layers=5]",
    "jak:sparg_arena_capa[layers=6]",
    "jak:sparg_arena_capa[layers=7]",
];

pub fn plan(
    center: (i32, i32, i32),
    turn: u8,
    size: &mut dyn FnMut(i32, bool) -> (i32, i32),
) -> Town {
    plan_seeded(0, center, turn, size)
}
pub fn plan_seeded(
    seed: i64,
    center: (i32, i32, i32),
    turn: u8,
    size: &mut dyn FnMut(i32, bool) -> (i32, i32),
) -> Town {
    let mut t = Town {
        center,
        turn,
        ring: Vec::new(),
        lots: Vec::new(),
        station: None,
        tower: None,
        palace: None,
        heights: Vec::new(),
        spacious: false,
    };
    let site = t.site();
    for (ix, x) in [-49i32, -35, -21, -7, 7, 21, 35, 49]
        .into_iter()
        .enumerate()
    {
        for (iz, z) in [-49i32, -35, -21, -7, 7, 21, 35, 49]
            .into_iter()
            .enumerate()
        {
            // A central plaza, a market square and a quiet courtyard.
            if x.abs() == 7 && z.abs() == 7 || (ix == 1 && iz == 2) || (ix == 6 && iz == 5) {
                continue;
            }
            let block_x = ix / 2;
            let block_z = iz / 2;
            let mut h = seed as u64
                ^ (center.0 as u64).rotate_left(19)
                ^ (center.2 as u64).rotate_left(37)
                ^ ((block_x as u64) * 7919 + (block_z as u64) * 104729);
            h ^= h >> 30;
            h = h.wrapping_mul(0xbf58476d1ce4e5b9);
            h ^= h >> 27;
            // Central districts always connect; the outskirts branch and end
            // differently in every settlement instead of repeating one square.
            let central = (1..=2).contains(&block_x) && (1..=2).contains(&block_z);
            if !central && h % 5 == 0 {
                continue;
            }
            let corner = (ix == 0 || ix == 7) && (iz == 0 || iz == 7);
            let (w, d) = size(11, corner);
            let front = if iz % 2 == 0 { 0 } else { 2 };
            t.lots.push(Lot {
                origin: site.at(x, 0, z),
                front: site.way(front),
                w: w.clamp(5, 11),
                d: d.clamp(5, 10),
                corner,
            });
        }
    }
    for side in 0..4 {
        let s = Site {
            origin: center,
            front: (turn + side) % 4,
            w: 0,
            d: 0,
        };
        for x in -56..56 {
            let (x, _, z) = s.at(x, 0, -56);
            t.ring.push((x, z));
        }
    }
    t
}
#[cfg(test)]
mod tests {
    use super::*;

    /// The ring is a loop (each point beside the next), the lots do not
    /// overlap, every lot's front is on the ring's outer edge.
    #[test]
    fn a_town_loops_and_its_lots_stand_apart() {
        for turn in 0..4 {
            let mut k = 0;
            let town = plan((100, 64, -40), turn, &mut |max, corner| {
                k += 1;
                (if corner { 7 } else { (5 + k % 5).min(max) }, 5 + k % 3)
            });
            let n = town.ring.len();
            for i in 0..n {
                let (a, b) = (town.ring[i], town.ring[(i + 1) % n]);
                assert!((a.0 - b.0).abs() + (a.1 - b.1).abs() <= 2, "{a:?} {b:?}");
            }
            let mut taken = std::collections::HashSet::new();
            for lot in &town.lots {
                let s = Site {
                    origin: lot.origin,
                    front: lot.front,
                    w: lot.w,
                    d: lot.d,
                };
                for x in 0..lot.w {
                    for z in 0..lot.d {
                        let (wx, _, wz) = s.at(x, 0, z);
                        assert!(taken.insert((wx, wz)), "overlap at {wx} {wz}");
                    }
                }
            }
            assert!(town.lots.len() >= 16, "{}", town.lots.len());
        }
    }
}

#[cfg(test)]
mod urban_tests {
    use super::*;
    #[test]
    fn bulk_survey_matches_the_original_column_queries() {
        use minecraftoss_core::{ChunkPos,chunk::HeightmapKind,registries::DataPaths};
        let Ok(paths)=DataPaths::discover() else{return;};
        if !paths.block_catalog.is_file(){return;}
        let registries=std::sync::Arc::new(minecraftoss_core::Registries::load(&paths).unwrap());
        let terrain=minecraftoss_generator::terrain::TerrainGenerator::overworld(registries,12345).unwrap();
        for pos in [ChunkPos::new(0,0),ChunkPos::new(-82,32)] {
            let tile=terrain.base_heightmaps(pos);
            for (x,z) in [(0,0),(7,3),(8,15),(15,15)] {for (k,kind) in [HeightmapKind::OceanFloorWg,HeightmapKind::WorldSurfaceWg].into_iter().enumerate(){
                assert_eq!(tile[z*16+x][k],terrain.base_height(pos.min_block_x()+x as i32,pos.min_block_z()+z as i32,kind),"{pos:?} {x} {z} {kind:?}");
            }}
        }
    }
    struct Flat;
    impl Land for Flat {
        fn ground(&self, _: i32, _: i32) -> i32 {
            70
        }
        fn wet(&self, _: i32, _: i32) -> bool {
            false
        }
    }
    #[test]
    fn settlements_change_with_the_seed_but_repeat_across_chunks() {
        let signature = |seed| {
            let t = plan_seeded(seed, (100, 70, -40), 0, &mut |_, _| (9, 8));
            t.lots.iter().map(|l| l.origin).collect::<Vec<_>>()
        };
        assert_eq!(signature(123), signature(123));
        assert_ne!(signature(123), signature(124));
        assert!(signature(123).len() >= 36);
    }
    #[test]
    fn streets_use_eighths_and_buildings_keep_level_entrances() {
        for seed in [1, 5, 87] {
            let mut t = plan_seeded(seed, (0, 70, 0), 0, &mut |_, _| (9, 8));
            t.grade(seed, &Flat, false);
            let mut lo = i32::MAX;
            let mut hi = i32::MIN;
            for x in -57..=57 {
                for z in -57..=57 {
                    let h = t.eighths(x, z);
                    lo = lo.min(h);
                    hi = hi.max(h);
                    if matches!(t.ground(x, z), Ground::Road | Ground::Street) {
                        for (dx, dz) in [(1, 0), (0, 1)] {
                            let delta = (h - t.eighths(x + dx, z + dz)).abs();
                            assert!(delta <= 4, "seed {seed}, {x} {z}: {delta}/8");
                        }
                    }
                }
            }
            assert!(hi - lo >= 16, "actual relief required");
            for lot in &t.lots {
                let local = lot.origin;
                assert_eq!(t.eighths(local.0, local.2), local.1 * 8);
            }
        }
    }
    #[test]
    fn a_building_never_straddles_the_wrong_biome() {
        struct Edge;
        impl Land for Edge {
            fn ground(&self, _: i32, _: i32) -> i32 {
                70
            }
            fn wet(&self, _: i32, _: i32) -> bool {
                false
            }
            fn city_allowed(&self, x: i32, _: i32, _: bool) -> bool {
                x < 20
            }
        }
        let mut t = plan_seeded(4, (0, 70, 0), 0, &mut |_, _| (11, 10));
        t.grade(4, &Edge, false);
        assert!(!t.lots.is_empty());
        for l in t.lots {
            let site = Site {
                origin: l.origin,
                front: l.front,
                w: l.w,
                d: l.d,
            };
            for x in -2..l.w + 2 {
                for z in -3..l.d + 2 {
                    assert!(site.at(x, 0, z).0 < 20);
                }
            }
        }
    }
    #[test]
    fn spargus_vehicle_streets_keep_connected_patrol_routes() {
        struct Flat;
        impl Land for Flat {fn ground(&self,_:i32,_:i32)->i32{70}fn wet(&self,_:i32,_:i32)->bool{false}}
        for turn in 0..4 {
            let mut town=plan_seeded(12345,(100,70,-200),turn,&mut|_,_|(12,12));
            town.spread_spargus(12345);town.grade(12345,&Flat,true);
            let civic=town.civic(true,&Flat);
            let links:std::collections::HashSet<_>=civic.iter().filter(|(_,s)|s.starts_with(ROUTE_MARK)).map(|(p,s)|{
                let way=if s.contains("east"){1}else if s.contains("south"){2}else if s.contains("west"){3}else{0};
                ((p.0,p.2),way)
            }).collect();
            assert!(links.len()>=16,"missing street network facing {turn}: {}",links.len());
            for &((x,z),way) in &links {
                let (dx,dz)=crate::jak_facades::dir_step(way);
                assert!(links.contains(&((x+dx*28,z+dz*28),(way+2)%4)),"route must join a reciprocal junction");
            }
        }
    }
    #[test]
    fn complete_settlements_keep_their_decoration_in_their_biome() {
        struct Edge;
        impl Land for Edge {
            fn ground(&self, _: i32, _: i32) -> i32 {
                70
            }
            fn wet(&self, _: i32, _: i32) -> bool {
                false
            }
            fn city_allowed(&self, x: i32, _: i32, _: bool) -> bool {
                x < 20
            }
        }
        for turn in 0..4 {
            for spargus in [false, true] {
                let blocks = if spargus {
                    crate::jak_spargus::town(4, (-48, 0), 70, turn, &Edge)
                } else {
                    crate::jak_barrio::village(4, (-48, 0), 70, turn, &Edge)
                };
                assert!(!blocks.is_empty());
                assert!(blocks.iter().all(|((x, _, _), _)| *x < 20));
            }
        }
    }
}


#[cfg(test)]mod finish_regressions{
 use super::*;
 #[test]fn haven_frontages_share_party_walls_and_vary_their_block_orientation(){
  for turn in 0..4 {for seed in 0..5 {
   let mut t=plan_seeded(seed,(100,70,-60),turn,&mut|_,_|(9,8));t.haven_frontages(seed);
   let mut cells=std::collections::HashMap::new();
   for(i,lot)in t.lots.iter().enumerate(){let s=Site{origin:lot.origin,front:lot.front,w:lot.w,d:lot.d};for x in 0..lot.w{for z in 0..lot.d{let p=s.at(x,0,z);assert!(cells.insert((p.0,p.2),i).is_none(),"overlap");}}}
   let contacts=cells.iter().filter(|&(&(x,z),i)|[(x+1,z),(x,z+1)].iter().any(|p|cells.get(p).is_some_and(|j|i!=j))).count();
   assert!(contacts>50,"Connected facades, {contacts} shared wall cells");
   let directions:std::collections::HashSet<_>=t.lots.iter().map(|l|l.front).collect();assert!(directions.len()>2);
  }}
 }
}


/// Seeded, read-only descriptions of complete original props. Generation
/// only populates this cache; player removals use jak_objects' world log.
type PropCache=std::collections::HashMap<(i64,(i32,i32)),std::sync::Arc<crate::jak_hamlets::Made>>;
static PROPS:std::sync::Mutex<Option<PropCache>>=std::sync::Mutex::new(None);
static FOLIAGE:std::sync::Mutex<Option<PropCache>>=std::sync::Mutex::new(None);

pub fn known_props_near(seed:i64,pos:(i32,i32,i32),reach:i32)->Vec<crate::jak_objects::Instance> {
    PROPS.lock().ok().and_then(|held|held.as_ref().map(|cache|cache.iter()
        .filter(|((s,(x,z)),_)|*s==seed&&(*x-pos.0).abs()<=reach+REACH&&(*z-pos.2).abs()<=reach+REACH)
        .flat_map(|(_,made)|made.instances.iter().copied()).collect())).unwrap_or_default()
}

fn prop_occupancy(town:&Town,out:&[Placed])->(std::collections::HashSet<(i32,i32,i32)>,std::collections::HashSet<(i32,i32,i32)>) {
    let actual:std::collections::HashSet<_>=out.iter().filter(|(_,s)|!s.ends_with(":air")).map(|(p,_)|*p).collect();
    let legacy=if town.tower.is_some() {
        let mut cells:std::collections::HashSet<_>=out.iter().filter(|(_,s)|!s.ends_with(":air")&&!s.starts_with("jak:pieza_haven_torre[")).map(|(p,_)|*p).collect();
        if let Some(lot)=town.lots.iter().find(|l|town.tower==Some((l.origin.0,l.origin.2))) {
            cells.extend(crate::jak_tower::legacy_building(lot.origin).into_iter().filter(|(_,s)|!s.ends_with(":air")).map(|(p,_)|p));
        }
        cells
    } else {actual.clone()};
    (legacy,actual)
}

fn foliage_wall(state:&str)->bool {
    let path=state.split('[').next().unwrap_or(state).strip_prefix("jak:").unwrap_or(state);
    matches!(crate::jak_families::kind(path),Some(crate::jak_families::Kind::Wall))
        && (path.ends_with("_pared") || path.ends_with("_pared_base"))
}

/// Only the new source identity uses these anchors. The old prop table is
/// still reconstructed unchanged for chunks saved with pieza_haven.
fn facade_foliage(town:&Town,seed:i64,land:&dyn Land,out:&mut Vec<Placed>,occupied:&mut std::collections::HashSet<(i32,i32,i32)>)->Vec<crate::jak_objects::Instance> {
    use crate::jak_objects::{self as objects,OBJECTS};
    let states:std::collections::HashMap<_,_>=out.iter().copied().collect();
    let doors:Vec<_>=states.iter().filter(|(_,s)|s.contains("half=lower")).map(|(p,_)|*p).collect();
    let mut instances=Vec::new();
    for (i,lot) in town.lots.iter().enumerate() {
        if town.station==Some((lot.origin.0,lot.origin.2)) || town.tower==Some((lot.origin.0,lot.origin.2)) {continue;}
        let s=Site{origin:lot.origin,front:lot.front,w:lot.w,d:lot.d};
        let index=(seed as u64).wrapping_add(i as u64*7919) as usize;
        let block=["objeto_haven_colgante_1","objeto_haven_colgante_2","objeto_haven_colgante_3"][index%3];
        let kind=OBJECTS.iter().position(|o|o.block==block).unwrap();
        'side: for n in 0..3 {
            let side=1+(n+index)%3;
            let (x,z)=match side {1=>(lot.w-1,lot.d/2),2=>(lot.w/2,lot.d-1),_=>(0,lot.d/2)};
            let (wx,_,wz)=s.at(x,0,z);
            let mut walls:Vec<_>=states.iter().filter_map(|(&(x,y,z),state)|
                (x==wx && z==wz && y>lot.origin.1 && foliage_wall(state)).then_some((x,y,z))).collect();
            walls.sort_unstable_by_key(|p|std::cmp::Reverse(p.1));
            let turns=(s.way(side as u8)+2)%4;
            for wall in walls {
                let Some(instance)=objects::hanging_at(kind,wall,turns) else{continue;};
                if !objects::supported_wall(&instance,|p|if states.get(&p).is_some_and(|s|foliage_wall(s)) {
                    vec![[0.0,0.0,0.0,1.0,1.0,1.0]]
                } else {Vec::new()}) {continue;}
                let cells=instance.cells();
                if cells.is_empty() || cells.iter().any(|(p,_)|occupied.contains(p)
                    || (p.0-town.center.0).abs()>REACH || (p.2-town.center.2).abs()>REACH
                    || !land.city_allowed(p.0,p.2,false) || land.wet(p.0,p.2)
                    || {let(x,z)=town.local(*p);p.1*8<town.eighths(x,z)+24}
                    || doors.iter().any(|d|(d.0-p.0).abs()<=3 && (d.2-p.2).abs()<=3)) {continue;}
                for (p,_) in cells {
                    out.push((p,"jak:pieza_haven_follaje[type=double,waterlogged=false]"));occupied.insert(p);
                }
                instances.push(instance);
                break 'side;
            }
        }
        if instances.len()>=6 {break;}
    }
    instances
}

pub(crate) fn original_props(town:&Town,seed:i64,spargus:bool,land:&dyn Land,out:&mut Vec<Placed>) {
    use crate::jak_objects::{self as objects,OBJECTS};
    let (mut occupied,actual_occupied)=prop_occupancy(town,out);
    let doors:Vec<_>=out.iter().filter(|(_,s)|s.contains("half=lower")).map(|(p,_)|*p).collect();
    let mut candidates=Vec::new();
    // Reserve planted corners before small market props fill their footprint.
    // The full native crown needs five cells; a barrel-sized gap is too small.
    if spargus {for (x,z) in [(-14,-13),(14,13),(-14,13),(14,-13)]{
        let (wx,_,wz)=town.site().at(x,0,z);
        candidates.push(((wx,town.eighths(x,z).div_euclid(8),wz),Some("objeto_spargus_palmera_completa")));
    }}
    for lot in &town.lots {let s=Site{origin:lot.origin,front:lot.front,w:lot.w,d:lot.d};
        for (x,z) in [(-3,lot.d/2),(lot.w+2,lot.d/2),(lot.w/2,lot.d+2)] {candidates.push((s.at(x,0,z),None));}
    }
    for (x,z) in [(-12,8),(-12,-12),(12,-10),(12,12),(-8,12),(8,-12)] {let (wx,_,wz)=town.site().at(x,0,z);candidates.push(((wx,town.eighths(x,z).div_euclid(8),wz),None));}
    if !spargus {
        // Counters fit beneath the existing market canopies. Their complete
        // footprint goes through the same floor, doorway and road checks.
        for (x,z) in [(-35,-21),(35,21)] {
            let (wx,_,wz)=town.site().at(x,0,z);
            if town.station==Some((wx,wz)){continue;}
            candidates.push(((wx-4,town.eighths(x,z).div_euclid(8),wz-1),Some("objeto_haven_puesto")));
        }
        for lot in &town.lots {
            let s=Site{origin:lot.origin,front:lot.front,w:lot.w,d:lot.d};
            for (x,z) in [(-2,lot.d-2),(lot.w+1,lot.d-2)] {
                let (wx,_,wz)=s.at(x,0,z);let (lx,lz)=town.local((wx,0,wz));
                candidates.push(((wx,town.eighths(lx,lz).div_euclid(8),wz),Some("objeto_haven_propaganda")));
            }
        }
    }
    // Visit six districts before accepting another projector in one district.
    let mut spread=Vec::new();
    for (x,z) in [(-50,-50),(50,50),(-50,50),(50,-50),(0,-65),(0,65)]{
        let target=town.site().at(x,0,z);
        if let Some((index,_))=candidates.iter().enumerate().filter(|(_,(_,a))|*a==Some("objeto_haven_propaganda"))
            .min_by_key(|(_, (p,_))|(p.0-target.0).pow(2)+(p.2-target.2).pow(2)){
            spread.push(candidates.remove(index));
        }
    }
    let mut non_projectors=Vec::new();let mut rest=Vec::new();
    for c in candidates{if c.1==Some("objeto_haven_propaganda"){rest.push(c);}else{non_projectors.push(c);}}
    non_projectors.extend(spread);non_projectors.extend(rest);let candidates=non_projectors;
    let first_prop=out.len();
    let mut instances=Vec::new();
    let (mut ordinary,mut posts,mut stands)=(0,0,0);
    let mut projector_sites:Vec<(i32,i32,i32)>=Vec::new();
    for (i,(at,authored)) in candidates.into_iter().enumerate() {
        if match authored {None=>ordinary>=if spargus{18}else{14},Some("objeto_haven_propaganda")=>posts>=6,Some(_)=>stands>=2}{continue;}
            if authored==Some("objeto_haven_propaganda") && projector_sites.iter().any(|p|(p.0-at.0).pow(2)+(p.2-at.2).pow(2)<24*24){continue;}
        let h=(seed as u64).wrapping_add(i as u64*7919).wrapping_add(town.center.0 as u64);
        let name=authored.unwrap_or_else(||if spargus {match i%5 {0=>"objeto_spargus_palmera_completa",1=>"objeto_spargus_urna_alta",2=>"objeto_spargus_urna_baja",_=>"objeto_spargus_caja_mercado"}}
            else {match i%7 {0=>"objeto_haven_planta_1",1=>"objeto_haven_planta_2",_=>"objeto_haven_bidon_fuego"}});
        let Some(kind)=OBJECTS.iter().position(|o|o.block==name) else{continue;};
        let kind=objects::placement_kind(kind);
        let turns=if authored==Some("objeto_haven_puesto"){0}else{(h%4) as u8};
        let Some(instance)=objects::placed(kind,at,turns) else{continue;};
        let cells=instance.cells();
        if cells.is_empty() || cells.iter().any(|(p,_)|town.palace_owns(*p)||occupied.contains(p) || (p.0-town.center.0).abs()>REACH || (p.2-town.center.2).abs()>REACH
            || !land.city_allowed(p.0,p.2,spargus) || land.wet(p.0,p.2)
            || matches!({let(x,z)=town.local(*p);town.ground(x,z)},Ground::Road|Ground::Street)
            || doors.iter().any(|d|(d.0-p.0).abs()<=3 && (d.2-p.2).abs()<=3)) {continue;}
        let columns:std::collections::HashSet<_>=cells.iter().map(|(p,_)|(p.0,p.2)).collect();
        if columns.iter().any(|&(x,z)|{let (lx,lz)=town.local((x,0,z));town.eighths(lx,lz)!=at.1*8 || !occupied.contains(&(x,at.1-1,z))}) {continue;}
        let shape=objects::shape(kind).unwrap();
        // A larger new tower can cover an old prop. Preserve the old
        // instance/source identity, but emit no partial prop through it.
        let emit=name!="objeto_spargus_barril_1" && cells.iter().all(|(p,_)|!actual_occupied.contains(p));
        for (p,cell) in cells {let slab=objects::slab_type(&shape.cells[&cell]);
            let state=if name=="objeto_haven_bidon_fuego" {"jak:pieza_haven_fuego[type=double,waterlogged=false]"}else{match (spargus,slab) {
                (false,"top")=>"jak:pieza_haven[type=top,waterlogged=false]",(false,"double")=>"jak:pieza_haven[type=double,waterlogged=false]",(false,_)=>"jak:pieza_haven[type=bottom,waterlogged=false]",
                (true,"top")=>"jak:pieza_spargus[type=top,waterlogged=false]",(true,"double")=>"jak:pieza_spargus[type=double,waterlogged=false]",(true,_)=>"jak:pieza_spargus[type=bottom,waterlogged=false]",
            }};if emit {out.push((p,state));}occupied.insert(p);
        }
        instances.push(instance);
        match authored {None=>ordinary+=1,Some("objeto_haven_propaganda")=>{posts+=1;projector_sites.push(at);},Some(_)=>stands+=1}
    }
    if !spargus{instances.extend(crate::jak_bar::props(town,out));}
    // Legacy seed lookup must not move or reshape any saved piece block.
    // Omit unsupported vines only from the blocks of a newly generated
    // city; retain their old instance records and reserve those cells so
    // a new source never collides with an old one.
    let legacy_vines:std::collections::HashSet<_>=instances.iter().filter(|i|OBJECTS[i.kind as usize].hangs_from_wall())
        .flat_map(|i|i.cells().into_iter().map(|(p,_)|p)).collect();
    let mut index=0;
    out.retain(|(p,_)|{let keep=index<first_prop||!legacy_vines.contains(p);index+=1;keep});
    occupied.extend(actual_occupied);
    let foliage=if spargus{Vec::new()}else{facade_foliage(town,seed,land,out,&mut occupied)};
    {let mut held=FOLIAGE.lock().unwrap_or_else(|e|e.into_inner());let cache=held.get_or_insert_with(Default::default);
        if cache.len()>128 {cache.clear();}
        cache.insert((seed,(town.center.0,town.center.2)),std::sync::Arc::new(crate::jak_hamlets::Made::new(foliage)));}
    let mut held=PROPS.lock().unwrap_or_else(|e|e.into_inner());let cache=held.get_or_insert_with(Default::default);
    if cache.len()>128 {cache.clear();}
    cache.insert((seed,(town.center.0,town.center.2)),std::sync::Arc::new(crate::jak_hamlets::Made::new(instances)));
}

pub(crate) fn foliage_at(pos:(i32,i32,i32))->Option<(crate::jak_objects::Instance,(i32,i32,i32))> {
    let seed=crate::original_places::current_world()?.0;
    // Rebuild the same district cache on demand after loading a saved chunk.
    let _=original_at(pos);
    cached_foliage_at(seed,pos)
}

pub(crate) fn cached_foliage_at(seed:i64,pos:(i32,i32,i32))->Option<(crate::jak_objects::Instance,(i32,i32,i32))> {
    FOLIAGE.lock().ok()?.as_ref()?.iter().filter(|((s,(x,z)),_)|*s==seed&&(*x-pos.0).abs()<=REACH&&(*z-pos.2).abs()<=REACH)
        .find_map(|(_,made)|made.at(pos))
}

pub(crate) fn cached_original_at(seed:i64,pos:(i32,i32,i32))->Option<(crate::jak_objects::Instance,(i32,i32,i32))> {
    let held=PROPS.lock().ok()?;
    held.as_ref()?.iter().filter(|((s,(x,z)),_)|*s==seed && (*x-pos.0).abs()<=REACH && (*z-pos.2).abs()<=REACH).find_map(|(_,made)|made.at(pos))
}

pub(crate) fn original_at(pos:(i32,i32,i32))->Option<(crate::jak_objects::Instance,(i32,i32,i32))> {
    let (seed,registries)=crate::original_places::current_world()?;
    if let Some(found)=cached_original_at(seed,pos){return Some(found);}
    // The city was already generated; absence from its prop table is a
    // known empty cell. Do not regenerate it for each empty chunk/voxel.
    if PROPS.lock().ok()?.as_ref().is_some_and(|cache|cache.keys().any(|(s,(x,z))|
        *s==seed&&(*x-pos.0).abs()<=REACH&&(*z-pos.2).abs()<=REACH)){return None;}
    // A saved chunk may be read before its generator has made the prop
    // cache this session. Reconstruct from the same immutable seed once.
    static SEARCHED:std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<(i64,i32,i32)>>>=std::sync::LazyLock::new(||std::sync::Mutex::new(Default::default()));
    {let mut seen=SEARCHED.lock().ok()?;if !seen.insert((seed,pos.0>>4,pos.2>>4)){return None;}}
    crate::jak_structures::cache_city_props_near(registries,seed,pos);
    cached_original_at(seed,pos)
}

#[cfg(test)] mod original_prop_regressions {
    use super::*;
    struct Flat;impl Land for Flat{fn ground(&self,_:i32,_:i32)->i32{70}fn wet(&self,_:i32,_:i32)->bool{false}}
    /// Explicit read-only route aid for the real generated city used by
    /// the city probe; no game process or saved chunks are modified.
    #[test]
    #[ignore = "print native foliage anchors for the seed12345 city probe"]
    fn report_seed12345_haven_hanging_foliage() {
        let paths=minecraftoss_core::registries::DataPaths::discover().unwrap();
        let registries=std::sync::Arc::new(minecraftoss_core::Registries::load(&paths).unwrap());
        crate::original_places::bind_world(registries,12345);
        let _=original_at((-2792,80,-5531));
        let mut rows:Vec<_>=FOLIAGE.lock().unwrap().as_ref().unwrap().iter()
            .filter(|((seed,(x,z)),_)|*seed==12345&&(*x+2792).abs()<120&&(*z+5531).abs()<120)
            .flat_map(|((_,center),made)|made.instances.iter().map(move |i|(*center,*i)))
            .collect();
        rows.sort_unstable_by_key(|(center,i)|(*center,i.origin,i.kind,i.turns));
        assert!(!rows.is_empty(),"the real Haven must contain anchored native foliage");
        for(center,i)in rows {
            let root=i.wall_root().unwrap();let cells=i.cells();
            let lo=[cells.iter().map(|(p,_)|p.0).min().unwrap(),cells.iter().map(|(p,_)|p.1).min().unwrap(),cells.iter().map(|(p,_)|p.2).min().unwrap()];
            let hi=[cells.iter().map(|(p,_)|p.0+1).max().unwrap(),cells.iter().map(|(p,_)|p.1+1).max().unwrap(),cells.iter().map(|(p,_)|p.2+1).max().unwrap()];
            eprintln!("HAVEN_FOLIAGE {}",serde_json::json!({"center":center,"kind":crate::jak_objects::OBJECTS[i.kind as usize].block,"origin":i.origin,"turns":i.turns,"root":root,"bounds":[lo,hi]}));
        }
    }
    #[test] fn new_haven_foliage_has_real_wall_anchors_and_legacy_props_keep_their_lookup() {
        use crate::jak_objects::{self as objects,OBJECTS};
        let mut anchored=std::collections::HashSet::new();
        let mut legacy=0;
        for seed in [314159,12345,9876] {
            let blocks=crate::jak_barrio::village(seed,(0,0),70,0,&Flat);
            let states:std::collections::HashMap<_,_>=blocks.iter().copied().collect();
            for (&pos,&state) in &states {
                if !state.starts_with("jak:pieza_haven_follaje["){continue;}
                let(instance,cell)=cached_foliage_at(seed,pos).expect("versioned foliage source");
                assert!(OBJECTS[instance.kind as usize].versioned_foliage());
                assert!(objects::shape(instance.kind as usize).unwrap().cells.contains_key(&cell));
                assert!(cached_original_at(seed,pos).is_none(),"new source overlaps a legacy prop");
                assert!(objects::supported_wall(&instance,|p|if states.get(&p).is_some_and(|s|foliage_wall(s)){
                    vec![[0.0,0.0,0.0,1.0,1.0,1.0]]
                }else{Vec::new()}),"no real wall behind root: {:?}",instance.wall_root());
                anchored.insert((seed,instance.kind,instance.origin,instance.turns));
            }
            let old=PROPS.lock().unwrap().as_ref().unwrap()[&(seed,(0,0))].instances.clone();
            for instance in old.iter().filter(|i|OBJECTS[i.kind as usize].hangs_from_wall()) {
                legacy+=1;
                assert!(!OBJECTS[instance.kind as usize].versioned_foliage());
                for(pos,_)in instance.cells(){
                    assert_eq!(cached_original_at(seed,pos).unwrap().0,*instance);
                    assert!(!states.get(&pos).is_some_and(|s|s.starts_with("jak:pieza_haven[")),"fresh city emitted an unsupported legacy vine");
                }
            }
        }
        assert!(legacy>0,"regression fixture must include the formerly floating shrubs");
        assert!(!anchored.is_empty(),"retain native foliage on real facades");
        let mut town=plan_seeded(7,(0,70,0),0,&mut|_,_|(9,8));town.haven_frontages(7);
        let mut empty=Vec::new();let mut occupied=std::collections::HashSet::new();
        assert!(facade_foliage(&town,7,&Flat,&mut empty,&mut occupied).is_empty(),"no wall means no hanging plant");
    }
    #[test] fn expanded_tower_does_not_reinterpret_legacy_prop_locations() {
        let seed=340189;
        let mut town=plan_seeded(seed,(0,70,0),0,&mut|_,_|(9,8));
        town.haven_frontages(seed);town.reserve_tower(seed,&Flat);town.grade(seed,&Flat,false);
        let tower=town.lots.iter().find(|l|town.tower==Some((l.origin.0,l.origin.2))).unwrap();
        let mut old=town.terrain(&Flat,false);old.extend(town.civic(false,&Flat));
        let mut new=old.clone();
        old.extend(crate::jak_tower::legacy_building(tower.origin));
        new.extend(crate::jak_tower::building_on(seed,tower.origin,&Flat));
        let (old_occupancy,_)=prop_occupancy(&town,&old);
        let (replayed,actual)=prop_occupancy(&town,&new);
        assert_eq!(old_occupancy,replayed,"new geometry moved the historical prop rejection mask");
        assert_ne!(old_occupancy,actual,"fixture must exercise different tower geometry");
        original_props(&town,seed,false,&Flat,&mut old);
        let before=PROPS.lock().unwrap().as_ref().unwrap()[&(seed,(0,0))].instances.clone();
        let first=new.len();
        original_props(&town,seed,false,&Flat,&mut new);
        let after=PROPS.lock().unwrap().as_ref().unwrap()[&(seed,(0,0))].instances.clone();
        assert!(!before.is_empty());assert_eq!(before,after,"saved prop identities changed beside the new tower");
        assert!(new[first..].iter().all(|(p,_)|!actual.contains(p)),"new props replaced actual tower blocks");
    }
    #[test] fn urban_props_are_complete_original_objects_off_the_traffic_lanes() {
        for (spargus,seed) in [(false,314159),(true,271828)] {
            let blocks=if spargus{crate::jak_spargus::town(seed,(0,0),70,0,&Flat)}else{crate::jak_barrio::village(seed,(0,0),70,0,&Flat)};
            let objects:Vec<_>=blocks.iter().filter(|(_,s)|s.starts_with("jak:pieza_")).collect();
            assert!(!objects.is_empty(),"original props in {spargus}");
            for &(at,state) in &objects {
                if state.starts_with("jak:pieza_haven_torre[") {
                    assert!(!spargus);
                    let(place,cell)=crate::jak_tower::source_at_expanded_seed(seed,*at).expect("expanded tower source");
                    assert_eq!(place,crate::jak_pieces::TOWER_SOURCE);assert!(crate::jak_tower::source_cell(place,cell));continue;
                }
                if state.starts_with("jak:pieza_haven_follaje[") {
                    assert!(!spargus);
                    let(instance,cell)=cached_foliage_at(seed,*at).expect("whole anchored foliage");
                    assert!(crate::jak_objects::OBJECTS[instance.kind as usize].versioned_foliage());
                    assert!(crate::jak_objects::shape(instance.kind as usize).unwrap().cells.contains_key(&cell));continue;
                }
                if let Some((place,cell))=crate::jak_tower::source_at_seed(seed,*at){
                    assert!(!spargus);assert_eq!(place,1);assert!(crate::jak_tower::source_cell(place,cell));continue;
                }
                let (instance,cell)=cached_original_at(seed,*at).expect("every cell belongs to a whole original object");
                assert_eq!(crate::jak_objects::OBJECTS[instance.kind as usize].place,if spargus{"spargus"}else{"haven"});
                assert!(crate::jak_objects::shape(instance.kind as usize).unwrap().cells.contains_key(&cell));
            }
        }
    }
    #[test] fn haven_has_original_street_furniture_without_blocking_doorways() {
        let mut found=std::collections::HashSet::new();
        for seed in [314159,12345,9876] {
            let blocks=crate::jak_barrio::village(seed,(0,0),70,0,&Flat);
            let doors:Vec<_>=blocks.iter().filter(|(_,s)|s.contains("half=lower")).map(|(p,_)|p).collect();
            for (at,state) in &blocks {
                if !state.starts_with("jak:pieza_haven["){continue;}
                if crate::jak_tower::source_at_seed(seed,*at).is_some(){continue;}
                let(instance,_)=cached_original_at(seed,*at).unwrap();
                let block=crate::jak_objects::OBJECTS[instance.kind as usize].block;
                if crate::original_places::street_actor(block).is_none(){continue;}
                found.insert(block);
                assert!(doors.iter().all(|d|(d.0-at.0).abs()>3||(d.2-at.2).abs()>3));
            }
        }
        assert!(found.contains("objeto_haven_puesto"),"no original market counter generated");
        assert!(found.contains("objeto_haven_propaganda"),"no original propaganda post generated");
    }
}

/// The explicit debug build moves its objects together with its blocks.
/// This runs on the requesting player's thread, using the ordinary saved
/// object placement path, rather than writing a world log in generation.
pub fn relocate_original_props(seed:i64,centre:(i32,i32),delta:(i32,i32,i32)) {
    crate::jak_tower::relocate(seed,centre,delta);
    let mut instances=PROPS.lock().ok().and_then(|p|p.as_ref()?.get(&(seed,centre)).map(|m|m.instances.clone())).unwrap_or_default();
    instances.retain(|i|!crate::jak_objects::OBJECTS[i.kind as usize].hangs_from_wall());
    instances.extend(FOLIAGE.lock().ok().and_then(|p|p.as_ref()?.get(&(seed,centre)).map(|m|m.instances.clone())).unwrap_or_default());
    for mut i in instances {i.origin=(i.origin.0+delta.0,i.origin.1+delta.1,i.origin.2+delta.2);let _=crate::jak_objects::add(i);}
}

#[cfg(test)] mod spargus_open_layout_tests {
    use super::*;
    #[test] fn architecture28_spargus_has_three_squares_and_clear_nine_block_roads() {
        for turn in 0..4 { for seed in [1,5,12345] {
            let mut town=plan_seeded(seed,(0,64,0),turn,&mut |_,_|(9,8));
            let dense=town.lots.len(); town.spread_spargus(seed);
            assert_eq!(town.lots.len(),18); assert!(town.lots.len()<dense);
            for (x,z,radius) in [(0,0,22),(-14,-42,10),(14,42,10)] {
                for dx in -radius..=radius { for dz in -radius..=radius {
                    assert_eq!(town.ground(x+dx,z+dz),Ground::Plaza);
                }}
            }
            for lot in &town.lots {
                let site=Site{origin:lot.origin,front:lot.front,w:lot.w,d:lot.d};
                for x in -1..=lot.w { for z in -1..=lot.d {
                    let (x,z)=town.local(site.at(x,0,z));
                    assert_eq!(town.ground(x,z),Ground::Lot,"building or trim obstructs circulation at {x},{z}");
                }}
            }
            for x in -60..=60 {for z in -4..=4 {
                assert!(matches!(town.ground(x,z),Ground::Road|Ground::Plaza));
            }}
        }}
    }
}

#[cfg(test)]mod wet_paving_tests {
    use super::*;
    struct Pocket{wet:std::collections::HashSet<(i32,i32)>}
    impl Land for Pocket{
        fn ground(&self,x:i32,z:i32)->i32{if self.wet.contains(&(x,z)){60}else{70}}
        fn wet(&self,x:i32,z:i32)->bool{self.wet.contains(&(x,z))}
    }
    #[test]fn urban_wet_pockets_have_a_solid_visible_foundation_but_the_shore_is_preserved(){
        for turn in 0..4{for spargus in [false,true]{
            let town=plan_seeded(5151,(0,70,0),turn,&mut|_,_|(9,8));let site=town.site();
            let edge=(-REACH..=REACH).flat_map(|x|(-REACH..=REACH).map(move|z|(x,z))).find(|&(x,z)|{let c=town.coverage(x,z);c>0.02&&c<=0.75}).unwrap();
            let e=site.at(edge.0,0,edge.1);let core=site.at(0,0,0);
            let land=Pocket{wet:[(core.0,core.2),(e.0,e.2)].into_iter().collect()};
            let blocks:std::collections::HashMap<_,_>=town.terrain(&land,spargus).into_iter().collect();
            for y in 60..70 {assert_eq!(blocks[&(core.0,y,core.2)],if spargus{"jak:sparg_losa"}else{"jak:barrio_losa"},"real masonry below the pavement, turn={turn}");}
            assert!(!blocks.keys().any(|&(x,_,z)|x==e.0&&z==e.2),"outer water shore must remain natural");
        }}
    }
    #[test]fn normal_jungle_city_does_not_leave_the_recorded_wet_hole_beside_the_house(){
        use std::sync::Arc;use minecraftoss_core::{Registries,registries::DataPaths};
        let r=Arc::new(Registries::load(&DataPaths::discover().unwrap()).unwrap());
        let gen=minecraftoss_world::chunk_map::WorldGen::new(Arc::new(minecraftoss_generator::terrain::TerrainGenerator::overworld(r,20261101).unwrap())).unwrap();
        assert!(gen.wet(20028,14815),"recorded real terrain regression");
        let blocks=crate::jak_structures::village_styled(20261101,(20010i32.div_euclid(320),14814i32.div_euclid(320)),&gen,crate::jak_structures::Style::Slums).unwrap();
        let final_blocks:std::collections::HashMap<_,_>=blocks.into_iter().collect();
        assert_eq!(final_blocks.get(&(20028,70,14815)),Some(&"jak:barrio_losa"));
        for y in 61..70{assert_eq!(final_blocks.get(&(20028,y,14815)),Some(&"jak:barrio_losa"));}
    }
}

#[cfg(test)]mod palace_district_regressions{
 use super::*;
 struct Flat;impl Land for Flat{fn ground(&self,_:i32,_:i32)->i32{70}fn wet(&self,_:i32,_:i32)->bool{false}}
 #[test]fn palace_has_real_buildings_and_clear_approaches_in_all_city_rotations(){
  for turn in 0..4{
   let town=crate::jak_barrio::natural_layout(0x1002026,(1600,-1600),70,turn,&Flat).unwrap();
   let c=town.palace.expect("flat complete district supports the palace");
   assert_eq!(c,town.center);assert!(town.lots.len()>=20);assert!(town.station.is_some());
   for (axis,sign) in [(0,-1),(0,1),(1,-1),(1,1)]{
    assert!(town.lots.iter().any(|l|{let d=if axis==0{l.origin.0-c.0}else{l.origin.2-c.2};d*sign>=68}),"missing buildings on {axis}/{sign}");
   }
   let city=crate::jak_barrio::village_from_layout(0x1002026,town.clone(),&Flat);
   let occupied:std::collections::HashSet<_>=city.iter().filter(|(_,s)|*s!="minecraft:air").map(|(p,_)|*p).collect();
   for (p,s) in crate::jak_city_arena::fixtures(){
    let at=(c.0+p.0,c.1+p.1,c.2+p.2);
    if s!="minecraft:air"{assert!(!occupied.contains(&at),"city overwrites palace {at:?}: {s}");}
   }
   for (dx,dz) in [(-57,0),(47,0),(0,-57),(0,47)] {
    assert!(occupied.contains(&(c.0+dx,c.1-1,c.2+dz)),"the precinct must keep its paved floor");
   }
   for l in &town.lots{
    assert!(city.iter().any(|(p,s)|p.1>l.origin.1+1&&(p.0-l.origin.0).abs()<=l.w&&(p.2-l.origin.2).abs()<=l.d&&*s!="minecraft:air"),"empty facade plot");
   }
   // The city and the final foot of both authored approaches share height.
   for (dx,dz) in [(-5,65),(67,-10)]{let(x,z)=town.local((c.0+dx,c.1,c.2+dz));assert_eq!(town.eighths(x,z),c.1*8);}
   assert!(city.iter().any(|(_,s)|s.starts_with("jak:bar_rotulo")),"the surrounding district keeps its bar");
  }
 }
 #[test]fn palace_never_crosses_a_single_block_strip_of_another_biome(){
  struct Border;impl Land for Border{fn ground(&self,_:i32,_:i32)->i32{70}fn wet(&self,_:i32,_:i32)->bool{false}fn city_allowed(&self,x:i32,_:i32,_:bool)->bool{x!=57}}
  let town=crate::jak_barrio::natural_layout(0x1002028,(0,0),70,0,&Border).unwrap();
  assert!(town.palace.is_none());
 }
 #[test]fn palace_is_not_detached_when_one_side_has_no_buildable_land(){
  struct Coast;impl Land for Coast{fn ground(&self,x:i32,_:i32)->i32{if x>45{10}else{70}}fn wet(&self,x:i32,_:i32)->bool{x>45}}
  let town=crate::jak_barrio::natural_layout(0x1002027,(0,0),70,0,&Coast).unwrap();
  assert!(town.palace.is_none(),"no fallback tower outside a one-sided city");
 }
}

pub fn export_foliage(seed:i64)->Vec<crate::jak_objects::Instance>{
 FOLIAGE.lock().ok().and_then(|held|held.as_ref().map(|cache|cache.iter().filter(|((s,_),_)|*s==seed).flat_map(|(_,made)|made.instances.iter().copied()).collect())).unwrap_or_default()
}
