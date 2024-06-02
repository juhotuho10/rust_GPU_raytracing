mod buffers;
mod camera;
mod image_texture;
mod renderer;
mod triangle_object;

use buffers::Params;
use camera::Camera;

use renderer::Renderer;

mod define_scene;

use define_scene::define_render_scene;

use triangle_object::SceneObject;

use egui::{pos2, Color32, DragValue, Frame, FullOutput};

use wgpu::{
    include_wgsl, Adapter, Backends, BindGroup, Device, Dx12Compiler, Gles3MinorVersion, Instance,
    InstanceDescriptor, InstanceFlags, PipelineLayout, Queue, Surface, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages,
};

use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, Event, KeyEvent, MouseButton, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window},
};

use egui_wgpu_backend::{RenderPass as EguiRenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};

use std::time::Instant;

pub fn main() {
    let event_loop = EventLoop::new().expect("failed to make eventloop");

    let builder = winit::window::WindowBuilder::new();

    // window width is set at 1600, because GPU buffer requires n * 256 bytes (n * 64 pixels * 4*u8 colors ) for every horisontal row,
    // changing it to not be a multiple of 64 requires implementing buffer values when getting colors from the GPU
    let window_size = PhysicalSize::new(1600, 900);

    let window = builder
        .with_inner_size(window_size)
        .build(&event_loop)
        .expect("failed to make window");

    window.set_resizable(false);
    env_logger::init();
    pollster::block_on(run(event_loop, window));
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    let mut movement_mode = false;

    let mut size = window.inner_size();

    let mut mouse_resting_position = egui::pos2(
        (size.width as f32 / 2.).round(),
        (size.height as f32 / 2.).round(),
    );

    let mut current_mouse_pos = mouse_resting_position;

    let mut show_ui = true;

    let mut compute_counter: u32 = 0;
    let mut compute_per_second: u32 = 0;

    let camera = Camera::new(size.width, size.height);

    let scene: renderer::RenderScene = define_render_scene();

    let mut last_mouse_pos: egui::Pos2 = pos2(0., 0.);

    let frametime_target = 5; // milliseconds

    let computetime_target = 0.8; // milliseconds

    let computation_per_frame = 5;

    let instance = generate_instance();

    let surface: Surface = instance
        .create_surface(&window)
        .expect("failed to make a surface");
    let adapter = create_adapter(&instance, &surface).await;
    // Create the logical device and command queue
    let (device, queue) = generate_device_and_queue(&adapter).await;

    let triangle_count = scene
        .objects
        .iter()
        .map(|obj: &SceneObject| obj.object_triangles.len())
        .sum::<usize>() as u32;

    let sub_object_count = scene
        .objects
        .iter()
        .map(|obj: &SceneObject| obj.sub_object_info.len())
        .sum::<usize>() as u32;

    println!("the following numbers should be the same in the compute shader for the buffers");
    dbg!(triangle_count);
    dbg!(sub_object_count);
    dbg!(scene.spheres.len());
    dbg!(scene.objects.len());
    dbg!(scene.materials.len());

    // Create uniform buffer
    let params = Params {
        screen_width: size.width,
        accumulation_index: 1,
        sky_color: scene.sky_color,
        accumulate: 1,
        sphere_count: scene.spheres.len() as u32,
        object_count: scene.objects.len() as u32,
        compute_per_frame: computation_per_frame,
        texture_width: scene.texture_size[0],
        texture_height: scene.texture_size[1],
        textue_count: scene.image_textures.len() as u32,
    };

    let (mut scene_renderer, compute_bindgroup_layout, compute_bind_group) =
        Renderer::new(camera, scene, &device, &queue, size, params);

    // ################################ GPU COMPUTE PIPELINE #########################################

    let compute_module = device.create_shader_module(include_wgsl!("compute_shader.wgsl"));

    let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Compute Pipeline Layout"),
        bind_group_layouts: &[&compute_bindgroup_layout],
        push_constant_ranges: &[],
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute Pipeline"),
        layout: Some(&compute_pipeline_layout),
        module: &compute_module,
        entry_point: "main",
        compilation_options: wgpu::PipelineCompilationOptions::default(),
    });

    // #####################################################################################
    // ################################ RENDER PIPELINE #########################################

    let mut texture = create_texture(&device, size);

    let sampler: wgpu::Sampler = generate_sampler(&device);

    let (mut bind_group_layout, mut bind_group) =
        create_device_bindgroup(&device, &texture, &sampler);

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let render_pipeline = create_render_pipeline(&device, &pipeline_layout, texture.format());

    let mut surface_config = wgpu::SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: texture.format(),
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Immediate,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![texture.format()],
    };

    surface.configure(&device, &surface_config);

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

    let mut egui_rpass = EguiRenderPass::new(&device, surface_config.format, 1);

    let mut fps_timer = Instant::now();

    let mut compute_timer = Instant::now();

    let mut compute_per_second_timer = Instant::now();

    /* ################################################################################ */

    //event_loop.set_control_flow(ControlFlow::Poll);
    event_loop
        .run(/*move*/ |event, target| {
            // Have the closure take ownership of the resources.

            platform.handle_event(&event);
            let _ = (&instance, &pipeline_layout);

            match event {
                Event::DeviceEvent { .. } => {
                    window.request_redraw();
                }
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::CursorMoved { position, .. } => {
                            if movement_mode {
                                current_mouse_pos = pos2(position.x as f32, position.y as f32);
                            }
                        }

                        WindowEvent::Resized(new_size) => {
                            size.width = new_size.width.max(1);
                            size.height = new_size.height.max(1);

                            surface_config.width = size.width;
                            surface_config.height = size.height;

                            screen_descriptor.physical_height = size.height;
                            screen_descriptor.physical_width = size.width;

                            mouse_resting_position = egui::pos2(
                                (size.width as f32 / 2.).round(),
                                (size.height as f32 / 2.).round(),
                            );

                            scene_renderer.on_resize(&size);

                            texture = create_texture(&device, size);

                            (bind_group_layout, bind_group) =
                                create_device_bindgroup(&device, &texture, &sampler);

                            surface.configure(&device, &surface_config);

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
                            window.request_redraw();
                        }

                        WindowEvent::MouseInput {
                            state,
                            button: MouseButton::Right,
                            ..
                        } => match state {
                            ElementState::Pressed => {
                                let grabbed = window.set_cursor_grab(CursorGrabMode::Confined);

                                let possible_pos = platform
                                    .context()
                                    .input(|i: &egui::InputState| i.pointer.hover_pos());

                                match (grabbed, possible_pos) {
                                    (Ok(_), Some(pos)) => {
                                        movement_mode = true;

                                        window.set_cursor_visible(false);

                                        last_mouse_pos = pos;
                                        current_mouse_pos = mouse_resting_position;
                                    }
                                    (Err(error), _) => {
                                        println!("cound not grab the cursor, {}", error)
                                    }

                                    (_, _) => println!("could not find cursor position"),
                                }

                                window.request_redraw();
                            }
                            ElementState::Released => {
                                // Logic when right mouse button is released

                                let grab_release = window.set_cursor_grab(CursorGrabMode::None);
                                window.set_cursor_visible(true);

                                let pos_set = window.set_cursor_position(PhysicalPosition::new(
                                    last_mouse_pos.x as u32,
                                    last_mouse_pos.y as u32,
                                ));

                                match (grab_release, pos_set) {
                                    (Ok(_), _) => movement_mode = false,
                                    (Err(error), _) => {
                                        println!("could not release cursor, {}", error)
                                    }
                                }

                                window.request_redraw();
                            }
                        },

                        WindowEvent::RedrawRequested => {
                            //println!(
                            //    "time 1: {}",
                            //    start_time.elapsed().as_micros() as f32 / 1000.
                            //);

                            // #############################################################################################

                            if compute_timer.elapsed().as_micros() as f32 / 1000.0
                                > computetime_target
                            {
                                compute_timer = Instant::now();
                                compute_counter += computation_per_frame;
                                scene_renderer
                                    .compute_frame(&compute_pipeline, &compute_bind_group);
                            }

                            if fps_timer.elapsed().as_millis() > frametime_target {
                                fps_timer = Instant::now();

                                if movement_mode {
                                    let _ = window.set_cursor_position(PhysicalPosition::new(
                                        mouse_resting_position.x,
                                        mouse_resting_position.y,
                                    ));
                                }

                                let mut encoder = device.create_command_encoder(
                                    &wgpu::CommandEncoderDescriptor {
                                        label: Some("Encoder"),
                                    },
                                );

                                scene_renderer.update_texture(&mut encoder, &texture);

                                // #############################################################################################

                                //println!(
                                //    "time 3: {}",
                                //    start_time.elapsed().as_micros() as f32 / 1000.
                                //);

                                // Logic to redraw the window
                                let frame: wgpu::SurfaceTexture = surface
                                    .get_current_texture()
                                    .expect("Failed to acquire next swap chain texture");

                                let view: wgpu::TextureView = frame
                                    .texture
                                    .create_view(&wgpu::TextureViewDescriptor::default());

                                //let pixel_colors = scene_renderer.generate_pixels();

                                setup_renderpass(
                                    &mut encoder,
                                    &view,
                                    &render_pipeline,
                                    &bind_group,
                                );

                                if compute_per_second_timer.elapsed().as_millis() > 1000 {
                                    compute_per_second_timer = Instant::now();
                                    compute_per_second = compute_counter;
                                    compute_counter = 0;
                                }

                                let full_output = create_ui(
                                    &mut platform,
                                    &mut scene_renderer,
                                    &compute_per_second,
                                );

                                let paint_jobs = platform
                                    .context()
                                    .tessellate(full_output.shapes, full_output.pixels_per_point);
                                // ######### Adding egui renderpass to the encoder ###########
                                if show_ui {
                                    egui_rpass
                                        .add_textures(&device, &queue, &full_output.textures_delta)
                                        .expect("couldnt add textures");

                                    egui_rpass.update_buffers(
                                        &device,
                                        &queue,
                                        &paint_jobs,
                                        &screen_descriptor,
                                    );

                                    egui_rpass
                                        .execute(
                                            &mut encoder,
                                            &view,
                                            &paint_jobs,
                                            &screen_descriptor,
                                            None,
                                        )
                                        .expect("egui render pass failed");
                                }
                                // ######### rendering the queue ###########
                                queue.submit(Some(encoder.finish()));

                                frame.present();

                                //println!(
                                //    "time 4: {}",
                                //    start_time.elapsed().as_micros() as f32 / 1000.
                                //);

                                egui_rpass
                                    .remove_textures(full_output.textures_delta)
                                    .expect("textures could not be removed");

                                //-------------

                                if movement_mode {
                                    let delta = current_mouse_pos - mouse_resting_position;

                                    scene_renderer.on_update(delta, &platform.context());
                                }

                                if platform
                                    .context()
                                    .input(|i: &egui::InputState| i.key_pressed(egui::Key::F11))
                                {
                                    show_ui = !show_ui;
                                }
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
        })
        .expect("Eventloop failed");
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
    texture: &wgpu::Texture,
    sampler: &wgpu::Sampler,
) -> (wgpu::BindGroupLayout, BindGroup) {
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let texture_bind = 0;
    let sampler_bind = 1;

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Texture Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: texture_bind,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: sampler_bind,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: texture_bind,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            wgpu::BindGroupEntry {
                binding: sampler_bind,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
        label: Some("Texture Bind Group"),
    });

    (bind_group_layout, render_bind_group)
}

fn setup_renderpass(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    render_pipeline: &wgpu::RenderPipeline,
    bind_group: &BindGroup,
) {
    let mut rpass: wgpu::RenderPass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

fn create_render_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &PipelineLayout,
    swapchain_format: TextureFormat,
) -> wgpu::RenderPipeline {
    // Load the shaders from disk
    let shader = device.create_shader_module(include_wgsl!("render_shader.wgsl"));

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
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            // Request an adapter which can render to our surface
            compatible_surface: Some(surface),
        })
        .await
        .expect("Failed to find an appropriate adapter")
}

async fn generate_device_and_queue(adapter: &Adapter) -> (Device, Queue) {
    let adapter_limits = wgpu::Limits {
        max_storage_buffers_per_shader_stage: 6,
        ..wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits())
    };
    adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                required_limits: adapter_limits,
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

fn generate_instance() -> Instance {
    let instance_desc: wgpu::InstanceDescriptor = InstanceDescriptor {
        backends: Backends::VULKAN,
        flags: InstanceFlags::default(),
        dx12_shader_compiler: Dx12Compiler::default(),
        gles_minor_version: Gles3MinorVersion::default(),
    };

    wgpu::Instance::new(instance_desc)
}

// ######################### UI CREATION ########################################

fn create_ui(
    platform: &mut Platform,
    screne_renderer: &mut Renderer,
    compute_per_second: &u32,
) -> FullOutput {
    platform.begin_frame();

    // important, create a egui context, do not use platform.conmtext()
    let egui_context = platform.context();

    let mut style = (*egui_context.style()).clone();
    style.visuals.override_text_color = Some(Color32::from_rgb(200, 200, 200));
    egui_context.set_style(style);

    let transparent_frame = Frame::none().fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200));

    let mut interacted = false;

    egui::SidePanel::right("side_panel")
        .resizable(false)
        .frame(transparent_frame)
        .show(&egui_context, |ui| {
            ui.set_max_width(180.0);

            ui.label(format!("fps: {}", compute_per_second));

            ui.vertical_centered(|ui| {
                ui.label("sky color:");
                if ui
                    .color_edit_button_rgb(&mut screne_renderer.scene.sky_color)
                    .on_hover_text("color")
                    .changed()
                {
                    interacted = true;
                };

                if ui
                    .checkbox(&mut screne_renderer.accumulate, "light accumulation")
                    .changed()
                {
                    interacted = true;
                };

                ui.add_space(10.0);

                ui.vertical_centered_justified(|ui: &mut egui::Ui| {
                    ui.label("selected object:");
                    ui.add(
                        egui::Slider::new(
                            &mut screne_renderer.object_index,
                            0..=(screne_renderer.scene.objects.len() - 1),
                        )
                        .integer(),
                    );

                    let current_object =
                        &mut screne_renderer.scene.objects[screne_renderer.object_index];

                    let coordinates = &mut current_object.transformation;

                    ui.label("location:");
                    ui.horizontal(|ui| {
                        if create_drag_value!(ui, &mut coordinates[0], 0.1, -400.0..=400.0, "X: ") {
                            interacted = true;
                        }

                        if create_drag_value!(ui, &mut coordinates[1], 0.1, -400.0..=10.0, "Y: ") {
                            interacted = true;
                        }

                        if create_drag_value!(ui, &mut coordinates[2], 0.1, -400.0..=400.0, "Z: ") {
                            interacted = true;
                        }
                    });

                    ui.add_space(10.0);

                    let rotation = &mut current_object.rotation;

                    ui.label("rotation:");
                    ui.horizontal(|ui| {
                        if create_drag_value!(ui, &mut rotation[0], 1.0, -180.0..=180.0, "X: ") {
                            interacted = true;
                        }

                        if create_drag_value!(ui, &mut rotation[1], 1.0, -180.0..=180.0, "Y: ") {
                            interacted = true;
                        }

                        if create_drag_value!(ui, &mut rotation[2], 1.0, -180.0..=180.0, "Z: ") {
                            interacted = true;
                        }
                    });

                    // sliders for scale
                    ui.vertical_centered_justified(|ui: &mut egui::Ui| {
                        let object_size = &mut current_object.scale;

                        if create_drag_value!(ui, object_size, 0.01, 0.1..=100.0, "scale: ") {
                            interacted = true;
                        }
                    });

                    ui.vertical_centered_justified(|ui: &mut egui::Ui| {
                        if ui.button("return to surface").clicked() {
                            current_object.set_model_to_surface();
                            interacted = true;
                        }
                    });

                    ui.vertical_centered_justified(|ui: &mut egui::Ui| {
                        if ui.button("reset rotation").clicked() {
                            current_object.reset_rotation();
                            interacted = true;
                        }
                    });

                    let material_index: usize = current_object.material_index as usize;
                    ui_material_selection(screne_renderer, material_index, ui, &mut interacted);
                });

                ui.add_space(30.0);

                ui.label("selected sphere:");
                ui.add(
                    egui::Slider::new(
                        &mut screne_renderer.sphere_index,
                        0..=(screne_renderer.scene.spheres.len() - 1),
                    )
                    .integer(),
                );

                let index = screne_renderer.sphere_index;
                let current_sphere = &mut screne_renderer.scene.spheres[index];

                // X Y Z sliders

                let sphere_position = &mut current_sphere.position;

                ui.horizontal(|ui| {
                    if create_drag_value!(ui, &mut sphere_position[0], 0.1, -400.0..=400.0, "X: ") {
                        interacted = true;
                    }

                    if create_drag_value!(ui, &mut sphere_position[1], 0.1, -400.0..=10.0, "Y: ") {
                        interacted = true;
                    }

                    if create_drag_value!(ui, &mut sphere_position[2], 0.1, -400.0..=400.0, "Z: ") {
                        interacted = true;
                    }
                });

                // sliders for radius
                ui.vertical_centered_justified(|ui: &mut egui::Ui| {
                    let sphere_radius = &mut current_sphere.radius;

                    if create_drag_value!(ui, sphere_radius, 0.01, 0.1..=50.0, "radius: ") {
                        interacted = true;
                    }
                });

                let material_index: usize = current_sphere.material_index as usize;
                ui_material_selection(screne_renderer, material_index, ui, &mut interacted);
            });
        });

    if interacted {
        screne_renderer.update_scene()
    }

    egui_context.end_frame()
}

fn ui_material_selection(
    screne_renderer: &mut Renderer,
    material_index: usize,
    ui: &mut egui::Ui,
    interacted: &mut bool,
) {
    ui.vertical_centered_justified(|ui: &mut egui::Ui| {
        ui.label("object material: ");
        let current_material = &mut screne_renderer.scene.materials[material_index];

        let texture_index = current_material.texture_index;

        let current_image = &mut screne_renderer.scene.image_textures[texture_index as usize];

        let emission_power = &mut current_material.emission_power;

        let color = &mut current_image.color;

        if let Some(color) = color {
            if ui
                .color_edit_button_rgb(color)
                .on_hover_text("color")
                .changed()
            {
                *interacted = true;
            };
        }

        if create_drag_value!(ui, emission_power, 0.2, 0.0..=200.0, "emission power: ") {
            *interacted = true;
        }

        let material_roughness = &mut current_material.roughness;

        if create_drag_value!(ui, material_roughness, 0.01, 0.0..=1.0, "roughness: ") {
            *interacted = true;
        }

        let material_specular = &mut current_material.specular;

        if create_drag_value!(ui, material_specular, 0.01, 0.0..=1.0, "specular: ") {
            *interacted = true;
        }

        let specular_scatter = &mut current_material.specular_scatter;

        if create_drag_value!(ui, specular_scatter, 0.01, 0.0..=0.5, "specular scatter: ") {
            *interacted = true;
        }

        let glass_refraction = &mut current_material.glass;

        if create_drag_value!(ui, glass_refraction, 0.01, 0.0..=1.0, "glass: ") {
            *interacted = true;
        }

        let refraction_index = &mut current_material.refraction_index;

        if create_drag_value!(ui, refraction_index, 0.01, 0.0..=5.0, "refraction index: ") {
            *interacted = true;
        }
    });
}

// simple macro for makíng the UI more compact
#[macro_export]
macro_rules! create_drag_value {
    ($ui:expr, $value:expr, $speed:expr, $range:expr, $prefix:expr) => {{
        if $ui
            .add(
                DragValue::new($value)
                    .speed($speed)
                    .clamp_range($range)
                    .prefix($prefix),
            )
            .changed()
        {
            true
        } else {
            false
        }
    }};
}
