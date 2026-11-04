//! Geometry shared by the mashup's visible pieces and native mob physics.
//! Coordinates use the block model's sixteenths; vanilla data stays intact.
pub fn haven_ditch_rim()->Vec<[f32;6]> {
    let mut boxes=vec![
        [0.,0.,2.,16.,2.5,9.],
        [0.,2.5,3.,16.,3.6,8.],
        [0.,-16.,-1.5,16.,2.,2.],
    ];
    for i in 0..6 {
        let z=-1.5-i as f32*2.4;
        boxes.push([0.,-16.,z-2.4,16.,-i as f32*2.6,z]);
    }
    boxes
}
