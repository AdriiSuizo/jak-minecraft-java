//! A small original Jak II slum tuft, reused in Minecraft leaf-litter portions.
//! Source: ctysluma/ctysluma-background.glb, material city-slum-cattail-grass
//! (MASK, doubleSided), ten triangles around [1101.292, 7.91019, 379.7466].
//! Positions are translated and uniformly scaled to fit a half-block portion;
//! their original UVs and topology stay intact. Its opaque texture is not a
//! billboard: replacing these triangles with crossed rectangles loses the grass.
use serde_json::json;

// Each vertex is [local x, y, z, u, v], in sixteenths. X/Z are centered.
const TUFT: [[[f32; 5]; 3]; 10] = [
    [[-1.0179900, 5.3360939, -0.5409576, 14.6796875, 7.0156250], [-1.5967528, 5.0721451, -0.3416212, 2.5546875, 7.0898438], [-1.9899265, 7.2483479, -1.6696137, 8.3125000, 0.1835938]],
    [[-0.7609149, 0.0200000, -0.1491586, 9.1484375, 15.4960938], [-1.0179900, 5.3360939, -0.5409576, 14.6796875, 7.0156250], [-1.5967528, 5.0721451, -0.3416212, 2.5546875, 7.0898438]],
    [[-0.7884096, 4.0300972, -0.7457928, 1.0625000, 7.6406250], [-1.8662005, 3.7743968, -1.1169708, 14.2304688, 7.8007812], [-2.0957810, 6.7369471, -2.5040768, 7.0195312, 0.2187500]],
    [[-0.7609149, 0.0200000, -0.1491586, 9.1484375, 15.4960938], [-0.7884096, 4.0300972, -0.7457928, 1.0625000, 7.6406250], [-1.8662005, 3.7743968, -1.1169708, 14.2304688, 7.8007812]],
    [[-1.9706803, 2.3721688, 0.0378052, 13.2968750, 7.4375000], [-1.2104527, 2.5948756, 0.5492060, 3.0429688, 7.7929688], [-2.9000000, 4.6610998, 0.9121356, 8.2304688, 0.7226562]],
    [[-0.7609149, 0.0200000, -0.1491586, 9.1484375, 15.4960938], [-1.9706803, 2.3721688, 0.0378052, 13.2968750, 7.4375000], [-1.2104527, 2.5948756, 0.5492060, 3.0429688, 7.7929688]],
    [[0.7196729, 3.3276084, 0.4351031, 14.7265625, 8.1523438], [0.4749704, 3.0086703, 1.5321403, 2.3085938, 8.3476562], [-0.7609149, 0.0200000, -0.1491586, 9.1523438, 15.4960938]],
    [[2.9000000, 5.4831903, 2.5040768, 7.8203125, 0.1835938], [0.7196729, 3.3276084, 0.4351031, 14.7265625, 8.1523438], [0.4749704, 3.0086703, 1.5321403, 2.3085938, 8.3476562]],
    [[0.5272102, 5.8461199, -0.0474283, 13.8359375, 6.9687500], [1.0111164, 5.4859398, 0.5890732, 3.4023438, 6.4843750], [-0.7609149, 0.0200000, -0.1491586, 9.1523438, 15.4960938]],
    [[2.6181797, 7.7185068, 0.2385162, 8.6718750, -0.0039062], [0.5272102, 5.8461199, -0.0474283, 13.8359375, 6.9687500], [1.0111164, 5.4859398, 0.5890732, 3.4023438, 6.4843750]],
];

pub(crate) const PARTS: [(&str, fn() -> String); 4] = [
    ("_1", one), ("_2", two), ("_3", three), ("_4", four),
];
fn one() -> String { portions(&[0]) }
fn two() -> String { portions(&[0, 1]) }
fn three() -> String { portions(&[2]) }
fn four() -> String { portions(&[0, 1, 2, 3]) }

// LeafLitterBlock north starts NW, then fills SW, SE and NE. Its vanilla
// multipart combines _2 + _3 for three portions and rotates the whole state.
fn portions(parts: &[usize]) -> String {
    let mut elements=Vec::new();
    for &part in parts {
        let (cx,cz)=[(4.0,4.0),(4.0,12.0),(12.0,12.0),(12.0,4.0)][part];
        for tri in TUFT {
            let positions=tri.map(|v| {
                let (x,z)=match part {0=>(v[0],v[2]),1=>(v[2],-v[0]),2=>(-v[0],-v[2]),_=>(-v[2],v[0])};
                [cx+x,v[1],cz+z]
            });
            let uv=tri.map(|v|[v[3],v[4]]);
            let from=std::array::from_fn::<_,3,_>(|axis|positions.iter().map(|p|p[axis]).fold(f32::INFINITY,f32::min));
            let to=std::array::from_fn::<_,3,_>(|axis|positions.iter().map(|p|p[axis]).fold(f32::NEG_INFINITY,f32::max));
            elements.push(json!({"from":from,"to":to,"shade_direction_override":"up",
                "faces":{"up":{"texture":"#g","iw4l_triangle":{"positions":positions,"uv":uv,"double_sided":true}}}}).to_string());
        }
    }
    elements.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_tufts_stay_in_their_leaf_litter_portions_with_original_uvs() {
        for (build,parts) in [(one as fn()->String,vec![0]),(two,vec![0,1]),(three,vec![2]),(four,vec![0,1,2,3])] {
            let elements:serde_json::Value=serde_json::from_str(&format!("[{}]",build())).unwrap();
            let elements=elements.as_array().unwrap();assert_eq!(elements.len(),10*parts.len());
            for (i,e) in elements.iter().enumerate() {
                let part=parts[i/10];let (cx,cz)=[(4.0,4.0),(4.0,12.0),(12.0,12.0),(12.0,4.0)][part];
                let t=&e["faces"]["up"]["iw4l_triangle"];
                assert_eq!(t["double_sided"],true);
                let positions=t["positions"].as_array().unwrap();assert_eq!(positions.len(),3);
                for v in positions {
                    let (x,y,z)=(v[0].as_f64().unwrap(),v[1].as_f64().unwrap(),v[2].as_f64().unwrap());
                    assert!((cx-4.0..cx+4.0).contains(&x)&&(cz-4.0..cz+4.0).contains(&z));
                    assert!((0.0..8.0).contains(&y),"grounded low tuft: {y}");
                }
                for (v,source) in t["uv"].as_array().unwrap().iter().zip(TUFT[i%10]) {
                    assert!((v[0].as_f64().unwrap()-f64::from(source[3])).abs()<1e-5);
                    assert!((v[1].as_f64().unwrap()-f64::from(source[4])).abs()<1e-5);
                }
                let p=positions.iter().map(|v|glam::Vec3::new(v[0].as_f64().unwrap() as f32,v[1].as_f64().unwrap() as f32,v[2].as_f64().unwrap() as f32)).collect::<Vec<_>>();
                assert!((p[1]-p[0]).cross(p[2]-p[0]).length_squared()>0.001,"original triangles must not collapse");
            }
            let base=elements.iter().flat_map(|e|e["faces"]["up"]["iw4l_triangle"]["positions"].as_array().unwrap()).map(|v|v[1].as_f64().unwrap()).fold(f64::INFINITY,f64::min);
            assert!((base-0.02).abs()<1e-5,"the source tuft is rooted on the surface");
        }
    }
}
