// Single source of truth: runs the same define_render_scene() used by the app at
// build time and exports the real scene counts to the crate.

#[path = "src/define_scene.rs"]
mod define_scene;
#[path = "src/image_texture.rs"]
mod image_texture;
#[path = "src/scene.rs"]
mod scene;
#[path = "src/triangle_object.rs"]
mod triangle_object;

use define_scene::define_render_scene;

fn main() {
    println!("cargo:rerun-if-changed=src/define_scene.rs");
    println!("cargo:rerun-if-changed=src/triangle_object.rs");
    println!("cargo:rerun-if-changed=src/scene.rs");
    println!("cargo:rerun-if-changed=3D_models");

    let scene = define_render_scene();

    println!(
        "cargo:rustc-env=TRIANGLE_COUNT={}",
        scene.total_triangle_count()
    );
    println!(
        "cargo:rustc-env=SUBOBJECT_COUNT={}",
        scene.total_sub_object_count()
    );
    println!("cargo:rustc-env=OBJECT_COUNT={}", scene.objects.len());
    println!("cargo:rustc-env=SPHERE_COUNT={}", scene.spheres.len());
    println!("cargo:rustc-env=MATERIAL_COUNT={}", scene.materials.len());
}
