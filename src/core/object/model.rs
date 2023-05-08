
use super::vertex::Vertex;
use std::io::Cursor;
use std::path::Path;

pub fn load<P: AsRef<Path>>(path: P) -> Cursor<Vec<u8>> {
    use std::fs::File;
    use std::io::Read;

    let mut buf = Vec::new();
    let fullpath = &Path::new("assets").join(&path);
    println!("Tentando acessar {:?}", fullpath);
    let mut file = File::open(&fullpath).unwrap();
    file.read_to_end(&mut buf).unwrap();
    Cursor::new(buf)
}

pub fn load_model(dir: &str, name: &str) -> (Vec<Vertex>, Vec<u32>) {
    let mut cursor = load(format!("{}/{}", dir, name));
    let (models, _) = tobj::load_obj_buf(
        &mut cursor,
        &tobj::LoadOptions {
            single_index: true,
            triangulate: true,
            ..Default::default()
        },
        |_| Ok((vec![], ahash::AHashMap::new())),
    ).unwrap();


    let mesh = &models[0].mesh;
    let positions = mesh.positions.as_slice();
    let coords = mesh.texcoords.as_slice();
    let vertex_count = mesh.positions.len() / 3;

    let mut vertices = Vec::with_capacity(vertex_count);
    for i in 0..vertex_count {
        let x = positions[i * 3];
        let y = positions[i * 3 + 1];
        let z = positions[i * 3 + 2];
        let u = coords[i * 2];
        let v = coords[i * 2 + 1];

        let vertex = Vertex {
            pos: [x, y, z],
            color: [1.0, 1.0, 1.0],
            coords: [u, v],
        };
        vertices.push(vertex);
    }

    (vertices, mesh.indices.clone())
} 