mod camera;

use camera::Camera;
use egui::{pos2, Frame, FullOutput};
use rand::{seq::index, thread_rng, Rng};
use rayon::{prelude::*, ThreadPoolBuilder};
use std::{borrow::Cow, time};
use wgpu::{
    Adapter, BindGroup, Device, PipelineLayout, Queue, Surface, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages,
};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window},
};

use egui_wgpu_backend::{RenderPass as EguiRenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use glam::{vec3a, Vec3A};

use std::time::Instant;

struct Ray {
    origin: Vec3A,
    direction: Vec3A,
}

pub fn main() {
    let event_loop = EventLoop::new().unwrap();

    let builder = winit::window::WindowBuilder::new();

    let window = builder.build(&event_loop).unwrap();

    env_logger::init();
    pollster::block_on(run(event_loop, window));
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    let mut rng = rand::thread_rng();

    let mut movement_mode = false;

    let available_threads = rayon::current_num_threads();
    let used_threads = available_threads / 2;

    let thread_pool = ThreadPoolBuilder::new()
        .num_threads(used_threads)
        .build()
        .unwrap();

    let mut size = window.inner_size();
    size.width = size.width.max(1);
    size.height = size.height.max(1);

    let mut camera = Camera::new(size.width, size.height);

    let mut last_mouse_pos: egui::Pos2 = pos2(0., 0.);

    let instance = wgpu::Instance::default();

    let surface: Surface = instance.create_surface(&window).unwrap();
    let adapter = create_adapter(&instance, &surface).await;
    // Create the logical device and command queue
    let (device, queue) = generate_device_and_queue(&adapter).await;

    let bind_group_layout = generate_bind_group_layout(&device);

    let mut texture = create_texture(&device, size);

    let sampler: wgpu::Sampler = generate_sampler(&device);

    let mut bind_group = create_device_bindgroup(&device, &bind_group_layout, &texture, &sampler);

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let swapchain_capabilities = surface.get_capabilities(&adapter);

    let render_pipeline = create_render_pipeline(&device, &pipeline_layout, swapchain_capabilities);

    let mut config = surface
        .get_default_config(&adapter, size.width, size.height)
        .unwrap();
    surface.configure(&device, &config);

    let window = &window;

    /* ################################ EGUI CODE ##################################### */
    // Initialize egui
    let scale_factor = window.scale_factor();

    let mut platform = Platform::new(PlatformDescriptor {
        physical_width: size.width,
        physical_height: size.height,
        scale_factor,
        font_definitions: Default::default(),
        style: Default::default(),
    });

    let mut screen_descriptor = ScreenDescriptor {
        physical_width: size.width,
        physical_height: size.height,
        scale_factor: scale_factor as f32,
    };

    let mut egui_rpass = EguiRenderPass::new(&device, config.format, 1);

    /* ##############################################3################################# */

    let mut temp_counter = 0;

    //event_loop.set_control_flow(ControlFlow::Poll);
    event_loop
        .run(/*move*/ |event, target| {
            // Have the closure take ownership of the resources.
            // `event_loop.run` never returns, therefore we must do this to ensure
            // the resources are properly cleaned up.

            let start_time = Instant::now();

            //println!("{:?}, counter: {}", event, temp_counter);

            let _ = (&instance, &pipeline_layout);

            platform.handle_event(&event);

            match event {
                Event::DeviceEvent { .. } => {
                    window.request_redraw();
                }
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::CursorMoved {
                            device_id: _,
                            position,
                        } => {
                            window.request_redraw();
                        }
                        WindowEvent::Resized(new_size) => {
                            size.width = new_size.width.max(1);
                            size.height = new_size.height.max(1);

                            config.width = size.width;
                            config.height = size.height;

                            screen_descriptor.physical_height = size.height;
                            screen_descriptor.physical_width = size.width;

                            camera.on_resize(size.width, size.height);

                            texture = create_texture(&device, size);

                            bind_group = create_device_bindgroup(
                                &device,
                                &bind_group_layout,
                                &texture,
                                &sampler,
                            );

                            surface.configure(&device, &config);
                            // On macos the window needs to be redrawn manually after resizing

                            window.request_redraw();
                        }

                        WindowEvent::CloseRequested => {
                            // Exit the application
                            target.exit();
                        }

                        WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    physical_key: PhysicalKey::Code(KeyCode::Space),
                                    repeat: false,
                                    state: ElementState::Pressed,
                                    ..
                                },
                            ..
                        } => {
                            // Logic for spacebar pressed
                            movement_mode = !movement_mode;
                            dbg!(movement_mode);
                            window.request_redraw();
                        }

                        WindowEvent::MouseInput {
                            state,
                            button: MouseButton::Right,
                            ..
                        } => match state {
                            ElementState::Pressed => {
                                movement_mode = true;

                                window
                                    .set_cursor_grab(CursorGrabMode::Confined)
                                    .expect("couldn't confine cursor");
                                window.set_cursor_visible(false);
                                /*let current_mouse_pos = platform
                                    .context()
                                    .input(|i: &egui::InputState| i.pointer.latest_pos())
                                    .unwrap();
                                last_mouse_pos = current_mouse_pos;*/
                                println!("cursor grabbed");
                                window.request_redraw();
                            }
                            ElementState::Released => {
                                // Logic when right mouse button is released
                                movement_mode = false;
                                window
                                    .set_cursor_grab(CursorGrabMode::None)
                                    .expect("Failed to release cursor");
                                window.set_cursor_visible(true);
                                window
                                    .set_cursor_position(PhysicalPosition::new(
                                        last_mouse_pos.x as u32,
                                        last_mouse_pos.y as u32,
                                    ))
                                    .expect("couldn't set cursor pos");
                                println!("cursor released");

                                dbg!(&camera.direction);
                                window.request_redraw();
                            }
                        },

                        WindowEvent::RedrawRequested => {
                            // Logic to redraw the window
                            let frame: wgpu::SurfaceTexture = surface
                                .get_current_texture()
                                .expect("Failed to acquire next swap chain texture");

                            let view: wgpu::TextureView = frame
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            let mut encoder: wgpu::CommandEncoder =
                                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: None,
                                });

                            let pixel_colors =
                                generate_pixels(&device, &camera, &mut rng, &thread_pool);

                            update_render_queue(&queue, &texture, &size, &pixel_colors);

                            setup_renderpass(&mut encoder, &view, &render_pipeline, &bind_group);
                            let full_output = create_ui(&mut platform);

                            /*let paint_jobs = platform
                                .context()
                                .tessellate(full_output.shapes, full_output.pixels_per_point);
                            // ######### Adding egui renderpass to the encoder ###########
                            egui_rpass
                                .add_textures(&device, &queue, &full_output.textures_delta)
                                .expect("couldnt add textures");

                            egui_rpass.update_buffers(&device, &queue, &paint_jobs, &screen_descriptor);

                            egui_rpass
                                .execute(&mut encoder, &view, &paint_jobs, &screen_descriptor, None)
                                .expect("egui render pass failed");*/

                            // ######### rendering the queue ###########
                            queue.submit(Some(encoder.finish()));

                            frame.present();

                            println!("new window drawn: {}", temp_counter);

                            //egui_rpass
                            //    .remove_textures(full_output.textures_delta)
                            //    .expect("textures removed");

                            //-------------

                            let elapsed = start_time.elapsed().as_micros() as f32 / 1000.;

                            if movement_mode {
                                //window
                                //    .set_cursor_position(PhysicalPosition::new(size.width / 2, size.height / 2))
                                //    .expect("couldn't set cursor pos");

                                let moved = camera.on_update(&elapsed, &platform.context());
                            }
                        }

                        _ => {
                            window.request_redraw();
                        } // Handle other window events that are not explicitly handled above
                    }
                }
                _ => {
                    window.request_redraw();
                } // Handle other types of events that are not window events
            }

            // ######### rendering the egui ###########

            window.request_redraw();
            temp_counter += 1;
        })
        .unwrap();
}

//#[allow(unused_variables)]
//fn generate_pixels(
//    device: &wgpu::Device,
//    camera: &Camera,
//    rng: &mut rand::rngs::ThreadRng,
//) -> Vec<u8> {
//    let camera_pos = camera.position;
//    let ray_directions = &camera.ray_directions;
//
//    let mut pixel_colors: Vec<u8> = Vec::with_capacity(ray_directions.len() * 4);
//
//    for index in 0..ray_directions.len() {
//        let ray = Ray {
//            origin: camera_pos,
//            direction: ray_directions[index],
//        };
//
//        let color = trace_ray(ray);
//
//        let color_rgba = to_rgba(color);
//        pixel_colors.extend_from_slice(&color_rgba);
//    }
//    pixel_colors
//}

#[allow(unused_variables)]
fn generate_pixels(
    device: &wgpu::Device,
    camera: &Camera,
    rng: &mut rand::rngs::ThreadRng,
    thread_pool: &rayon::ThreadPool,
) -> Vec<u8> {
    let camera_pos = camera.position;
    let ray_directions = &camera.ray_directions;

    let mut pixel_colors: Vec<u8> = Vec::with_capacity(ray_directions.len() * 4);

    thread_pool.install(|| {
        pixel_colors = (0..ray_directions.len())
            .into_par_iter()
            .flat_map_iter(|index| {
                // TODO: make a struct of ray consisting of origin and direction in the camera class

                let ray = Ray {
                    origin: camera_pos,
                    direction: ray_directions[index],
                };

                let color = trace_ray(ray);

                let color_rgba = to_rgba(color);

                color_rgba.into_iter()
            })
            .collect();
    });
    pixel_colors
}

fn update_render_queue(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    size: &winit::dpi::PhysicalSize<u32>,
    pixel_colors: &[u8],
) {
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixel_colors,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(size.width * 4),
            rows_per_image: Some(size.height),
        },
        wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
    );
}

fn create_texture(device: &wgpu::Device, size: winit::dpi::PhysicalSize<u32>) -> wgpu::Texture {
    let texture_size = wgpu::Extent3d {
        width: size.width.max(1),
        height: size.height.max(1),
        depth_or_array_layers: 1,
    };

    device.create_texture(&TextureDescriptor {
        label: None,
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn create_device_bindgroup(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    texture: &wgpu::Texture,
    sampler: &wgpu::Sampler,
) -> BindGroup {
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
        label: Some("Texture Bind Group"),
    })
}

fn setup_renderpass(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    render_pipeline: &wgpu::RenderPipeline,
    bind_group: &BindGroup,
) {
    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    rpass.set_pipeline(render_pipeline);
    rpass.set_bind_group(0, bind_group, &[]);
    rpass.draw(0..6, 0..1);
}

fn generate_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Texture Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn create_render_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &PipelineLayout,
    swapchain_capabilities: wgpu::SurfaceCapabilities,
) -> wgpu::RenderPipeline {
    // Load the shaders from disk
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
    });

    let swapchain_format = swapchain_capabilities.formats[0];

    let render_pipeline: wgpu::RenderPipeline =
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(swapchain_format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

    render_pipeline
}

async fn create_adapter(instance: &wgpu::Instance, surface: &Surface<'_>) -> wgpu::Adapter {
    instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            // Request an adapter which can render to our surface
            compatible_surface: Some(surface),
        })
        .await
        .expect("Failed to find an appropriate adapter")
}

async fn generate_device_and_queue(adapter: &Adapter) -> (Device, Queue) {
    adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
            },
            None,
        )
        .await
        .expect("Failed to create device")
}

fn generate_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    })
}

fn create_ui(platform: &mut Platform) -> FullOutput {
    platform.begin_frame();

    // important, create a egui context, do not use platform.conmtext()
    let egui_context = platform.context();

    let transparent_frame = Frame::none().fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200));

    egui::SidePanel::right("side_panel")
        .resizable(false)
        .frame(transparent_frame)
        .show(&egui_context, |ui| {
            ui.heading("Hello, world!");
            ui.label("This panel is on the right side.");

            ui.vertical_centered(|ui| {
                if ui.button("Hello").clicked() {
                    println!("Hello");
                }

                ui.add_space(5.0);

                ui.checkbox(&mut false, "clickme");
            });
        });

    /*if egui_context.input(|i: &egui::InputState| i.pointer.secondary_down()) {
        println!("right_click");
        egui::Context::send_viewport_cmd(
            &egui_context,
            egui::ViewportCommand::CursorGrab(egui::CursorGrab::Locked),
        );

        egui::Context::send_viewport_cmd(
            &egui_context,
            egui::ViewportCommand::CursorVisible(false),
        );

        egui::Context::send_viewport_cmd(
            &egui_context,
            egui::ViewportCommand::CursorPosition(pos2(100., 100.)),
        );
    } else {
        egui::Context::send_viewport_cmd(
            &egui_context,
            egui::ViewportCommand::CursorGrab(egui::CursorGrab::None),
        );
        egui::Context::send_viewport_cmd(&egui_context, egui::ViewportCommand::CursorVisible(true));
    }*/
    egui_context.end_frame()
}

fn trace_ray(ray: Ray) -> Vec3A {
    // (bx^2 + by^2)t^2 + 2*(axbx + ayby)t + (ax^2 + by^2 - r^2) = 0
    // where
    // a = ray origin
    // b = ray direction
    // r = sphere radius
    // t = hit distance

    //dbg!(ray.direction);

    let sphere_origin = vec3a(0., 0., 0.);
    let light_direction = vec3a(-1., -1., -1.).normalize();
    let radius: f32 = 0.5;

    let a: f32 = ray.direction.dot(ray.direction);
    let b: f32 = 2.0 * ray.direction.dot(ray.origin);
    let c: f32 = ray.origin.dot(ray.origin) - (radius * radius);

    // discriminant:
    // b^2 - 4*a*c
    let discriminant = b * b - 4. * a * c;

    if discriminant < 0. {
        // we missed the sphere
        return Vec3A::splat(0.);
    }
    // (-b +- discriminant) / 2a
    //let t0 = (-b + discriminant.sqrt()) / (2. * a);
    let closest_t = (-b - discriminant.sqrt()) / (2. * a);

    let hit_point = ray.origin + ray.direction * closest_t;

    let sphere_normal = (hit_point - sphere_origin).normalize();

    // cosine of the angle between hitpoin and the light direction
    // min light intenstiy is 0
    let light_intensity = sphere_normal.dot(-light_direction).max(0.05);

    vec3a(1., 0., 1.) * light_intensity
}

fn to_rgba(mut vector: Vec3A) -> [u8; 4] {
    vector *= 255.0;
    [vector.x as u8, vector.y as u8, vector.z as u8, 255]
}
