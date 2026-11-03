use super::{METER, Part, Particle, Sexp, Field,Value,keyword, random};
use glam::{Vec3, Vec4};
use std::collections::HashMap;

/// The two games share this callback name but launch different part IDs.
/// Read its actual 3D launcher rather than borrowing an ID from another game.
pub(super) fn water_drop_parts(forms:&[Sexp],name:&str)->Option<Vec<(u32,bool)>>{
    fn table_id(s:&Sexp)->Option<u32>{
        let Sexp::List(a)=s else{return None};
        if a.len()==3 && a[0].symbol()==Some("->") && a[1].symbol()==Some("*part-id-table*"){
            return a[2].number().filter(|n|n.is_finite()&&*n>=0.0&&n.fract()==0.0).map(|n|n as u32);
        }
        a.iter().find_map(table_id)
    }
    fn launch(s:&Sexp,out:&mut Vec<(u32,bool)>){
        let Sexp::List(a)=s else{return};
        if a.first().and_then(Sexp::symbol)==Some("launch-particles"){
            if let Some(id)=table_id(s){out.push((id,a.iter().any(|v|v.symbol()==Some("*sp-particle-system-3d*"))));}
        }else{for s in a{launch(s,out);}}
    }
    forms.iter().find_map(|s|{
        let Sexp::List(a)=s else{return None};
        if a.first().and_then(Sexp::symbol)==Some("defun")&&a.get(1).and_then(Sexp::symbol)==Some(name){let mut out=Vec::new();launch(s,&mut out);Some(out)}else{None}
    })
}

#[derive(Clone, Debug)]
pub(super) struct TextureSequence {
    period: f32,
    looping: bool,
    pub frames: Vec<u32>,
}

impl TextureSequence {
    pub fn read(args: &[Sexp]) -> Option<Self> {
        let data = args.iter().find_map(|arg| match arg {
            Sexp::List(values) if values.iter().any(|v| v.symbol() == Some("boxed-array")) => {
                Some(values)
            }
            _ => None,
        })?;
        let start = data.iter().position(|v| v.symbol() == Some("int32"))? + 1;
        let values: Vec<f32> = data.get(start..)?.iter().filter_map(Sexp::number).collect();
        let period = *values.first()?;
        let looping = *values.get(1)? != 0.0;
        let frames: Vec<_> = values.get(3..)?.iter().map(|v| *v as u32).collect();
        (!frames.is_empty() && period > 0.0).then_some(Self {
            period,
            looping,
            frames,
        })
    }

    pub fn texture(&self, age: f32, start: usize, names: &HashMap<u32, String>) -> Option<String> {
        let frame = start.saturating_add((age.max(0.0) / self.period) as usize);
        let frame = if self.looping {
            frame % self.frames.len()
        } else {
            frame.min(self.frames.len() - 1)
        };
        self.frames.get(frame).and_then(|id| names.get(id)).cloned()
    }
}

pub(super) fn spline(part: &Part) -> bool {
    matches!(
        part.live_func.as_deref(),
        Some(
            "sparticle-2d-spline-align-instant"
                | "sparticle-lightning-2d-spline-align-plus-rotz"
                | "spt-func-part-gun-dark-1-upgrade-shot-edges"
        )
    )
}

pub(super) fn supported(name: &str, birth: bool) -> bool {
    if birth {
        matches!(
            name,
            "birth-func-texture-group"
                | "birth-func-flip-based-on-scale"
                | "birth-func-set-vel" | "birth-func-set-vel-2d"
                | "spt-birth-func-part-buggy-rocks" | "spt-birth-func-part-dbuggy-debris"
                | "none"
                | "birth-func-target-orient"
                | "birth-func-camera-orient"
                | "birth-func-set-quat"
                | "birth-func-setup-beam"
                | "birth-func-laser-pointer"
                | "birth-func-curve"
                | "birth-func-set-alpha-from-userdata"
                | "birth-func-converge"
                | "birth-func-inherit-size"
                | "birth-func-ctywide-baron-propoganda-ticker-rotate"
        )
    } else {
        matches!(
            name,
            "sparticle-motion-blur" | "none"
                | "spt-func-turn-to-vel-radial"
                | "sparticle-texture-animate"
                | "sparticle-dark-shot-lightning"
                | "sparticle-track-root"
                | "part-tracker-track-root"
                | "board-zap-track" | "board-charge-track"
                | "sparticle-track-gun-joint"
                | "sparticle-track-gun-joint-player-y"
                | "sparticle-2d-spline-align-instant"
                | "sparticle-lightning-2d-spline-align-plus-rotz"
                | "spt-func-part-gun-dark-1-upgrade-shot-edges"
                | "live-func-curve"
                | "sparticle-red-2-glow-trail-halt"
                | "sparticle-red-2-converge"
                | "spt-func-relative-pos" | "check-scorp-shell-level1" | "check-scorp-shell-level2"
                | "check-shell-level1"
                | "check-shell-level2"
                | "eco-fadeout"
                | "eco-track-root-prim-fadeout"
                | "sparticle-track-root-prim"
                | "sparticle-3d-rotate-xz-to-camera"
                | "spt-func-part-vent-eco-dark-shaft"
                | "check-drop-group-center"
                | "spt-func-part-vent-eco-dark-touched-specs"
                | "sparticle-fade-alpha-dist"
                | "check-water-level-drop" | "check-water-level-drop-motion"
                | "check-water-level-drop-and-die" | "check-water-level-drop-and-die-motion"
                | "check-water-level-above-and-die"
        )
    }
}

/// ctywide-part.gc: the child's authored quaternion rotated around Y by
/// 45 degrees plus its launching sprite's Z rotation; inherit its alpha.
pub(super) fn bound_birth(library:&super::Library,p:&mut Particle,rotation:f32,alpha:f32,floor:Option<f32>) {
    if p.live_func.as_deref().is_some_and(|f|f.starts_with("check-water-level-")){p.floor=floor.or(p.floor);}
    if library.parts.get(&p.part).and_then(|part|part.birth_func.as_deref())
        ==Some("birth-func-ctywide-baron-propoganda-ticker-rotate") {
        p.quad=Some(glam::Mat3::from_rotation_y(std::f32::consts::FRAC_PI_4+rotation)
            *p.quad.unwrap_or(glam::Mat3::IDENTITY));
        p.rgba[3]=alpha;
    }
}

/// Whether a part's particles sit on their tracker's root each frame.
pub(super) fn tracks_root(func: Option<&str>) -> bool {
    // board-part.gc: board-zap-track reads the launching actor's joint 37.
    // The host supplies that joint as this tagged launch's moving root.
    matches!(func, Some("sparticle-track-root" | "part-tracker-track-root" | "sparticle-track-root-prim" | "eco-track-root-prim-fadeout" | "board-zap-track" | "board-charge-track"))
}

pub(super) fn update(
    _part: &Part,
    p: &mut Particle,
    roots: &HashMap<u64, Vec3>,
    target: Option<(Vec3, Vec3)>,
    camera: Option<Vec3>,
    dt: f32,
) {
    // The callback runs before the next-launcher countdown. Eco holds
    // that countdown while its process is alive, then lets it fade out.
    if matches!(p.live_func.as_deref(), Some("eco-fadeout" | "eco-track-root-prim-fadeout")) && !p.fading {
        p.next_time = Some(dt * 300.0 * 2.0);
    }
    match p.live_func.as_deref() {
        Some("spt-func-turn-to-vel-radial")=>{
            // sparticle-launcher.gc: velocity is forward; the XZ radial
            // vector from the owner's root, turned 90 degrees, is right.
            let root=p.source.and_then(|id|roots.get(&id)).copied().unwrap_or(p.origin);
            let radial=p.pos-root;let right=Vec3::new(radial.z,0.,-radial.x).normalize_or_zero();
            let forward=p.vel.normalize_or_zero();let up=forward.cross(right).normalize_or_zero();
            if right!=Vec3::ZERO&&forward!=Vec3::ZERO&&up!=Vec3::ZERO{p.quad=Some(glam::Mat3::from_cols(up.cross(forward).normalize(),up,forward));}
        },
        Some(
            "sparticle-track-root"
            | "part-tracker-track-root"
            | "board-zap-track" | "board-charge-track"
            | "sparticle-dark-shot-lightning"
            | "spt-func-part-gun-dark-1-upgrade-shot-edges"
            | "sparticle-track-root-prim"
            | "eco-track-root-prim-fadeout",
        ) => {
            p.pos = p
                .source
                .and_then(|id| roots.get(&id))
                .copied()
                .unwrap_or(p.origin);
        }
        Some("sparticle-track-gun-joint") => {
            if let Some((_, muzzle)) = target {
                p.pos = muzzle;
            }
        }
        Some("sparticle-track-gun-joint-player-y") => {
            if let Some((root, muzzle)) = target {
                p.pos = Vec3::new(muzzle.x, root.y + METER, muzzle.z);
            }
        }
        Some("sparticle-red-2-glow-trail-halt") => {
            let speed = p.vel.length();
            if speed > 0.0 {
                p.vel *= (1.0 - p.omega * dt / (300.0 * speed)).max(0.0);
            }
        }
        Some("sparticle-red-2-converge") => {
            if let (Some(dir), Some((_, muzzle))) = (p.converge, target) {
                p.pos = muzzle + dir.normalize_or(Vec3::Y) * (10240.0 * (p.timer / 450.0).powi(2));
            }
        }
        // `gun-blue-shot.gc`: alpha 128 at the eye, none 100 m away.
        Some("sparticle-fade-alpha-dist") => {
            if let Some(eye) = camera {
                p.rgba[3] = (128.0 * (1.0 - p.pos.distance(eye) / (100.0 * METER))).max(0.0);
            }
        }
        Some("check-drop-group-center" | "spt-func-part-vent-eco-dark-touched-specs") => {
            if p.pos.y < p.origin.y { p.timer = 0.0; }
        }
        Some("check-water-level-above-and-die")=>{if p.floor.is_some_and(|top|p.pos.y>=top){p.timer=0.0;}}
        _ => {}
    }
}

#[derive(Clone,Debug,Default)]
pub(super) struct TrackerSpecs {sets:HashMap<String,Vec<(u32,Field,bool,Sexp)>>,errors:HashMap<String,String>,specs:HashMap<u32,Vec<Sexp>>}
impl TrackerSpecs{
    pub fn read(&mut self,forms:&[Sexp]){
        self.specs.extend(forms.iter().filter_map(|f|{
            let Sexp::List(f)=f else{return None};
            if f.first()?.symbol()?!="defpart"{return None;}
            let id=f.get(1)?.number()?as u32;let Sexp::List(specs)=keyword(f,":init-specs")?else{return None};Some((id,specs.clone()))
        }));
        let specs=&self.specs;
        for f in forms{let Sexp::List(f)=f else{continue};
            if f.first().and_then(Sexp::symbol)!=Some("defun")||f.get(1).and_then(Sexp::symbol)!=Some("part-water-splash-callback"){continue;}
            let name="part-water-splash-callback";let mut sets=Vec::new();let mut error=None;
            fn walk(items:&[Sexp],specs:&HashMap<u32,Vec<Sexp>>,out:&mut Vec<(u32,Field,bool,Sexp)>,error:&mut Option<String>){
                if items.first().and_then(Sexp::symbol)==Some("set!"){
                    if let(Some(Sexp::List(path)),Some(value))=(items.get(1),items.get(2)){
                        if path.get(1).and_then(Sexp::symbol)==Some("*part-id-table*")&&path.get(3).and_then(Sexp::symbol)==Some("init-specs"){
                            if let(Some(id),Some(index),Some(member))=(path.get(2).and_then(Sexp::number),path.get(4).and_then(Sexp::number),path.get(5).and_then(Sexp::symbol)){
                                let key=specs.get(&(id as u32)).and_then(|s|s.get(index as usize)).and_then(|s|match s{Sexp::List(s)=>s.first().and_then(Sexp::symbol),_=>None});
                                if let Some(field)=key.and_then(|s|Field::parse(s.trim_start_matches(':'))){out.push((id as u32,field,member=="random-rangef",value.clone()));}
                                else{*error=Some(format!("unknown native init-spec {id}/{index}: {key:?}"));}
                            }
                        }
                    }
                }
                for item in items{if let Sexp::List(items)=item{walk(items,specs,out,error);}}
            }
            walk(f,&specs,&mut sets,&mut error);
            self.sets.insert(name.into(),sets);if let Some(e)=error{self.errors.insert(name.into(),e);}
        }
    }
    pub fn values(&self,name:&str,userdata:f32,root:Vec3,parts:&HashMap<u32,Part>)->Result<Vec<(u32,Field,f32,f32)>,String>{
        if let Some(e)=self.errors.get(name){return Err(e.clone());}
        let sets=self.sets.get(name).ok_or_else(||format!("unknown native tracker {name}"))?;
        fn eval(v:&Sexp,data:f32,y:f32)->Option<f32>{
            if let Some(n)=v.number(){return Some(n);}
            match v{
                Sexp::Atom(_)=>match v.symbol()?{"f0-0"=>Some(data),"f1-0"=>Some(y),_=>None},
                Sexp::List(a)=>{let op=a.first()?.symbol()?;let n=a[1..].iter().map(|v|eval(v,data,y)).collect::<Option<Vec<_>>>()?;
                    match op{"*"=>Some(n.iter().product()),"+"=>Some(n.iter().sum()),"-"=>Some(n[1..].iter().fold(*n.first()?,|a,b|a-b)),"/"=>Some(n[1..].iter().fold(*n.first()?,|a,b|a/b)),_=>None}}
            }
        }
        let mut values=HashMap::new();
        for(id,field,range,expr)in sets{
            let n=eval(expr,userdata,root.y).filter(|n|n.is_finite()).ok_or_else(||format!("unsupported native tracker expression: {expr:?}"))?;
            let original=parts.get(id).and_then(|p|p.fields.iter().rev().find(|(f,_)|f==field)).map(|(_,v)|*v);
            let pair=values.entry((*id,*field)).or_insert_with(||match original{Some(Value::Range(a,b))=>(a,b),_=>(0.0,0.0)});
            if *range{pair.1=n}else{pair.0=n}
        }
        Ok(values.into_iter().map(|((id,f),(a,b))|(id,f,a,b)).collect())
    }
}

#[derive(Clone, Debug)]
struct Curve {
    knots: [f32; 4],
    values: [Vec4; 4],
}

fn vector(value: &Sexp) -> Option<Vec4> {
    let Sexp::List(items) = value else {
        return None;
    };
    Some(Vec4::new(
        keyword(items, ":x").and_then(Sexp::number).unwrap_or(0.0),
        keyword(items, ":y").and_then(Sexp::number).unwrap_or(0.0),
        keyword(items, ":z").and_then(Sexp::number).unwrap_or(0.0),
        keyword(items, ":w").and_then(Sexp::number).unwrap_or(0.0),
    ))
}

impl Curve {
    fn read(items: &[Sexp]) -> Option<Self> {
        let xs = vector(keyword(items, ":xs")?)?;
        let ys = keyword(items, ":ys")?;
        let knots = (-xs).to_array();
        let mut values = [Vec4::ZERO; 4];
        if items.iter().any(|v| v.symbol() == Some("curve-color-fast")) {
            let Sexp::List(ys) = ys else { return None };
            for (out, value) in values
                .iter_mut()
                .zip(ys.iter().filter(|v| matches!(v, Sexp::List(_))))
            {
                *out = vector(value)?;
            }
        } else {
            for (out, v) in values.iter_mut().zip(vector(ys)?.to_array()) {
                *out = Vec4::splat(v);
            }
        }
        Some(Self { knots, values })
    }

    fn sample(&self, t: f32) -> Vec4 {
        let t = t.clamp(0.0, 1.0);
        for ((a, b), (x, y)) in self
            .knots
            .iter()
            .zip(self.knots.iter().skip(1))
            .zip(self.values.iter().zip(self.values.iter().skip(1)))
        {
            if t <= *b && b > a {
                return x.lerp(*y, ((t - a) / (b - a)).clamp(0.0, 1.0));
            }
        }
        self.values.last().copied().unwrap_or(Vec4::ZERO)
    }
}

#[derive(Clone, Debug, Default)]
struct Settings {
    lifetime: f32,
    offset: f32,
    square: bool,
    fields: HashMap<String, String>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Curves {
    curves: HashMap<String, Curve>,
    settings: HashMap<String, Settings>,
    parts: HashMap<u32, String>,
}

fn reference(value: &Sexp) -> Option<&str> {
    match value {
        Sexp::Atom(_) => value.symbol().filter(|s| *s != "#f"),
        Sexp::List(items) if items.first()?.symbol()? == "the-as" => items.last()?.symbol(),
        _ => None,
    }
}

impl Curves {
    pub fn read(&mut self, forms: &[Sexp]) {
        for form in forms {
            let Sexp::List(items) = form else { continue };
            if matches!(
                items.first().and_then(Sexp::symbol),
                Some("define" | "set!")
            ) {
                if let (Some(name), Some(Sexp::List(value))) =
                    (items.get(1).and_then(Sexp::symbol), items.get(2))
                {
                    if let Some(curve) = Curve::read(value) {
                        self.curves.insert(name.to_owned(), curve);
                    }
                    if value
                        .iter()
                        .any(|v| v.symbol() == Some("particle-curve-settings"))
                    {
                        let square = match keyword(value, ":flags") {
                            Some(Sexp::List(flags)) => {
                                flags.iter().any(|f| f.symbol() == Some("pcf0"))
                            }
                            _ => false,
                        };
                        self.settings.insert(
                            name.to_owned(),
                            Settings {
                                lifetime: keyword(value, ":lifetime-base")
                                    .and_then(Sexp::number)
                                    .unwrap_or(300.0),
                                offset: keyword(value, ":lifetime-offset")
                                    .and_then(Sexp::number)
                                    .unwrap_or(0.0),
                                square,
                                fields: HashMap::new(),
                            },
                        );
                    }
                }
                if let (Some(Sexp::List(path)), Some(value)) = (items.get(1), items.get(2)) {
                    if path.get(1).and_then(Sexp::symbol) == Some("*part-id-table*")
                        && path.iter().any(|v| v.symbol() == Some("initial-valuef"))
                    {
                        if let (Some(id), Some(name)) =
                            (path.get(2).and_then(Sexp::number), reference(value))
                        {
                            self.parts.insert(id as u32, name.to_owned());
                        }
                    } else if let (Some(name), Some(field), Some(value)) = (
                        path.get(1).and_then(Sexp::symbol),
                        path.get(2).and_then(Sexp::symbol),
                        reference(value),
                    ) {
                        if let Some(settings) = self.settings.get_mut(name) {
                            settings.fields.insert(field.to_owned(), value.to_owned());
                        }
                    }
                }
            }
            self.read(items);
        }
    }

    fn sample(&self, settings: &str, field: &str, t: f32) -> Option<Vec4> {
        let name = self.settings.get(settings)?.fields.get(field)?;
        Some(self.curves.get(name)?.sample(t))
    }
}

#[derive(Clone, Debug)]
pub(super) struct CurveState {
    settings: String,
    duration: f32,
    rgba: [f32; 4],
    size: [f32; 2],
}

impl CurveState {
    pub fn birth(curves: &Curves, id: u32, p: &mut Particle, seed: &mut u32) -> Option<Self> {
        let settings = curves.parts.get(&id)?;
        let config = curves.settings.get(settings)?;
        let duration = config.lifetime + (random(seed) * config.offset.max(0.0)).floor();
        let mut rgba = p.rgba;
        if let Some(c) = curves.sample(settings, "color-start", random(seed)) {
            rgba = c.to_array();
        }
        if let Some(a) = curves.sample(settings, "alpha-start", random(seed)) {
            if let Some(alpha) = rgba.get_mut(3) {
                *alpha = a.x;
            }
        }
        let mut size = p.scale;
        if let Some(x) = curves.sample(settings, "scale-x-start", random(seed)) {
            if let Some(width) = size.first_mut() {
                *width *= x.x;
            }
        }
        if config.square {
            if let Some(x) = size.first().copied() {
                if let Some(y) = size.get_mut(1) {
                    *y = x;
                }
            }
        } else if let Some(y) = curves.sample(settings, "scale-y-start", random(seed)) {
            if let Some(height) = size.get_mut(1) {
                *height *= y.x;
            }
        }
        let state = Self {
            settings: settings.clone(),
            duration: duration.max(1.0),
            rgba,
            size,
        };
        p.timer = state.duration;
        state.apply(curves, 0.0, &mut p.rgba, &mut p.scale);
        Some(state)
    }

    pub fn apply(&self, curves: &Curves, age: f32, rgba: &mut [f32; 4], scale: &mut [f32; 2]) {
        let t = (age / self.duration).clamp(0.0, 1.0);
        for ((out, start), field) in rgba
            .iter_mut()
            .zip(self.rgba)
            .zip(["r-scalar", "g-scalar", "b-scalar", "a-scalar"])
        {
            *out = (start * curves.sample(&self.settings, field, t).map_or(1.0, |v| v.x))
                .clamp(0.0, 255.0);
        }
        for ((out, start), field) in scale
            .iter_mut()
            .zip(self.size)
            .zip(["scale-x-scalar", "scale-y-scalar"])
        {
            *out = (start * curves.sample(&self.settings, field, t).map_or(1.0, |v| v.x)).abs();
        }
        if curves
            .settings
            .get(&self.settings)
            .is_some_and(|s| s.square)
        {
            if let Some(x) = scale.first().copied() {
                if let Some(y) = scale.get_mut(1) {
                    *y = x;
                }
            }
        }
    }
}
