use crate::app::*;
use crate::helpers::*;
use crate::intersect::*;
use crate::movement::*;
use crate::scene::*;
// use crate::sky_box::sky_box;

mod app;
mod bresenham;
mod helpers;
mod intersect;
mod movement;
mod scene;
mod sky_box;

use cgmath::{InnerSpace, Vector3};
use minifb::{Key, Window, WindowOptions};
use ordered_float::OrderedFloat;
use rand::prelude::*;
use rayon::prelude::*;

const WIDTH: usize = 400;
const HEIGHT: usize = 400;
const PIXEL_SIZE: f32 = 1.0 / WIDTH as f32;

fn main() {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut rgb_buffer: Vec<(Col)> = vec![Col::new(0.0, 0.0, 0.0); WIDTH * HEIGHT];
    let mut window = Window::new("", WIDTH, HEIGHT, WindowOptions::default()).unwrap_or_else(|e| {
        panic!("{}", e);
    });

    let mut scene = initialize_scene();

    let mut viewport = Viewport {
        overlays_enabled: true,
        distance_pass: false,
        sample_iter: 0,
        time: Time {
            prev: app::timestamp(),
            sum: 0.0,
            framecount: 0,
        },
    };

    let mut movement = Movement {
        camera_movement: Vector3::new(0.0, 0.0, 0.0),
        mouse_movement: Vector3::new(0.0, 0.0, 0.0),
        moving: false,
    };
    let mut keys_down: Vec<Key> = vec![];

    let uv_size = 2.0 * (rad(scene.cameras[0].fov / 2.0)).tan();

    // Main loop
    while window.is_open() && !window.is_key_down(Key::Escape) {
        app::update_time(
            &mut window,
            &mut viewport.time.prev,
            &mut viewport.time.framecount,
            &mut viewport.time.sum,
            &viewport.sample_iter,
        );

        let mut rot = cgmath::Matrix4::from_angle_z(cgmath::Rad(scene.cameras[0].rot.z))
            * cgmath::Matrix4::from_angle_y(cgmath::Rad(scene.cameras[0].rot.y))
            * cgmath::Matrix4::from_angle_x(cgmath::Rad(scene.cameras[0].rot.x));

        handle_movement(
            &mut window,
            &mut viewport,
            &mut scene.cameras[0],
            &mut rgb_buffer,
            &mut movement,
            &mut rot,
            &mut keys_down,
            &WIDTH,
            &HEIGHT,
        );

        let jitter_size =
            2.0 * scene.cameras[0].aperture_size * (1.0 - 1.0 / (scene.cameras[0].focus_distance));

        rgb_buffer
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, pixel)| {
                let mut rng = thread_rng();

                let jitter_angle = rng.gen_range(0.0, 1.0) * std::f32::consts::PI * 2.0;
                let jitter_length = (rng.gen_range(0.0, 1.0) as f32).sqrt() * PIXEL_SIZE;
                let jitter_x = jitter_length * jitter_angle.cos();
                let jitter_z = jitter_length * jitter_angle.sin();

                let aperture_jitter =
                    Vector3::new(jitter_x, 0.0, jitter_z) * 2.0 * scene.cameras[0].aperture_size;

                let aliasing_jitter = Vector3::new(
                    rng.gen_range(-1.0, 1.0) * PIXEL_SIZE,
                    0.0,
                    rng.gen_range(-1.0, 1.0) * PIXEL_SIZE,
                );

                let dir = {
                    let uv = uv(WIDTH * HEIGHT - i - 1);

                    Vector3::new(
                        ((uv.x - WIDTH as f32 / 2.0) / HEIGHT as f32) * -uv_size
                            + jitter_x * jitter_size,
                        1.0,
                        ((uv.y - HEIGHT as f32 / 2.0) / HEIGHT as f32) * uv_size
                            + jitter_z * jitter_size,
                    ) - aperture_jitter
                        + aliasing_jitter
                };

                let dir = rot * dir.extend(0.0);
                let dir = dir.truncate();

                let aperture_jitter_mat4 = rot * aperture_jitter.extend(0.0);
                let aperture_jitter = aperture_jitter_mat4.truncate();

                let ray = Ray {
                    pos: aperture_jitter + scene.cameras[0].pos + aliasing_jitter,
                    dir: dir.normalize(),
                };

                let col = intersect_spheres(2, 0, &scene, &viewport, &scene.spheres, &ray);

                pixel.r += col.r.powf(2.0);
                pixel.g += col.g.powf(2.0);
                pixel.b += col.b.powf(2.0);
            });

        viewport.sample_iter += 1;

        for (col_1, col_2) in rgb_buffer.iter().zip(buffer.iter_mut()) {
            let col = Col::new(
                clamp_max((col_1.r / viewport.sample_iter as f32).sqrt(), 1.0),
                clamp_max((col_1.g / viewport.sample_iter as f32).sqrt(), 1.0),
                clamp_max((col_1.b / viewport.sample_iter as f32).sqrt(), 1.0),
            );

            *col_2 = col_to_rgb_u32(col);
        }

        if viewport.overlays_enabled {
            for wireframe in &mut scene.wireframes {
                wireframe.render(&mut buffer, &scene.cameras[0], &WIDTH, &HEIGHT);
            }
        }

        window.update_with_buffer(&buffer).unwrap();
    }
}
