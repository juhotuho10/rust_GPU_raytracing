mod buffers;
mod camera;
mod renderer;

use buffers::{Params, SceneMaterial, SceneSphere};
use camera::Camera;

use egui::{pos2, Color32, DragValue, Frame, FullOutput};

use renderer::{RenderScene, Renderer};

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
    let event_loop = EventLoop::new().unwrap();

    let builder = winit::window::WindowBuilder::new();

    let window_size = PhysicalSize::new(1600, 900);

    let window = builder
        .with_inner_size(window_size)
        .build(&event_loop)
        .unwrap();

    window.set_resizable(false);
    env_logger::init();
    pollster::block_on(run(event_loop, window));
}

fn define_render_scene() -> RenderScene {
    let shiny_green = SceneMaterial {
        albedo: [0.1, 0.8, 0.4],
        roughness: 0.3,
        emission_color: [0.1, 0.8, 0.4],
        metallic: 1.0,
        emission_power: 0.0,
        _padding: [0; 12],
    };

    let rough_blue = SceneMaterial {
        albedo: [0.3, 0.2, 0.8],
        roughness: 0.7,
        emission_color: [0.3, 0.2, 0.8],
        metallic: 0.5,
        emission_power: 0.0,
        _padding: [0; 12],
    };

    let glossy_pink = SceneMaterial {
        albedo: [1.0, 0.1, 1.0],
        roughness: 0.4,
        emission_color: [1.0, 0.1, 1.0],
        metallic: 0.8,
        emission_power: 0.0,
        _padding: [0; 12],
    };

    let shiny_orange = SceneMaterial {
        albedo: [1.0, 0.7, 0.0],
        roughness: 0.7,
        emission_color: [1.0, 0.7, 0.0],
        metallic: 0.7,
        emission_power: 10.0,
        _padding: [0; 12],
    };

    let sphere_a: SceneSphere = SceneSphere {
        position: [0., -0.5, 0.],
        radius: 0.5,

        material_index: 2,
        _padding: [0; 12],
    };

    let sphere_b: SceneSphere = SceneSphere {
        position: [-3., -2.0, 3.],
        radius: 2.0,

        material_index: 0,
        _padding: [0; 12],
    };

    let shiny_sphere: SceneSphere = SceneSphere {
        position: [3., -15.0, -5.],
        radius: 7.0,

        material_index: 3,
        _padding: [0; 12],
    };

    // sphere to act as a floor
    let floor: SceneSphere = SceneSphere {
        position: [0., 500., 0.],
        radius: 500.,

        material_index: 1,
        _padding: [0; 12],
    };

    RenderScene {
        materials: vec![shiny_green, rough_blue, glossy_pink, shiny_orange],
        spheres: vec![sphere_a, sphere_b, shiny_sphere, floor],
        sky_color: [0., 0.04, 0.1],
    }
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    let mut movement_mode = false;

    let mut size = window.inner_size();
    size.width = size.width.max(1);
    size.height = size.height.max(1);

    let mut mouse_resting_position = egui::pos2(
        (size.width as f32 / 2.).round(),
        (size.height as f32 / 2.).round(),
    );

    let mut current_mouse_pos = mouse_resting_position;

    let mut show_ui = true;

    let camera = Camera::new(size.width, size.height);

    let scene = define_render_scene();

    let mut last_mouse_pos: egui::Pos2 = pos2(0., 0.);

    let instance = generate_instance();

    let surface: Surface = instance.create_surface(&window).unwrap();
    let adapter = create_adapter(&instance, &surface).await;
    // Create the logical device and command queue
    let (device, queue) = generate_device_and_queue(&adapter).await;

    // Create uniform buffer
    let params = Params {
        width: size.width,
        accumulation_index: 1,
        sky_color: scene.sky_color,
        accumulate: 1,
        _padding: [0; 8],
    };

    let (mut scene_renderer, compute_bindgroup_layout, compute_bind_group) =
        Renderer::new(camera, scene, &device, &size, params);

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

    /* ##############################################3################################# */

    //event_loop.set_control_flow(ControlFlow::Poll);
    event_loop
        .run(/*move*/ |event, target| {
            // Have the closure take ownership of the resources.

            // `event_loop.run` never returns, therefore we must do this to ensure
            // the resources are properly cleaned up.

            platform.handle_event(&event);
            let _ = (&instance, &pipeline_layout);

            match event {
                Event::DeviceEvent { .. } => {
                    window.request_redraw();
                }
                Event::WindowEvent { event, .. } => {
                    let start_time = Instant::now();

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

                            scene_renderer.on_resize(&size, &device, &queue);

                            texture = create_texture(&device, size);

                            (bind_group_layout, bind_group) =
                                create_device_bindgroup(&device, &texture, &sampler);

                            // ###################### COMPUTE RESIZING ##################################

                            // ####################################################

                            surface.configure(&device, &surface_config);
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

                                last_mouse_pos = platform
                                    .context()
                                    .input(|i: &egui::InputState| i.pointer.hover_pos())
                                    .unwrap();

                                current_mouse_pos = mouse_resting_position;

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

                                window.request_redraw();
                            }
                        },

                        WindowEvent::RedrawRequested => {
                            //println!(
                            //    "time 1: {}",
                            //    start_time.elapsed().as_micros() as f32 / 1000.
                            //);
                            if movement_mode {
                                window
                                    .set_cursor_position(PhysicalPosition::new(
                                        mouse_resting_position.x,
                                        mouse_resting_position.y,
                                    ))
                                    .expect("couldn't set cursor pos");
                            }

                            // #############################################################################################

                            let mut encoder =
                                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("Encoder"),
                                });

                            scene_renderer.update_frame(
                                &mut encoder,
                                &queue,
                                &compute_pipeline,
                                &compute_bind_group,
                                &texture,
                            );

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

                            setup_renderpass(&mut encoder, &view, &render_pipeline, &bind_group);

                            let full_output =
                                create_ui(&device, &queue, &mut platform, &mut scene_renderer);

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

                            //egui_rpass
                            //    .remove_textures(full_output.textures_delta)
                            //    .expect("textures removed");

                            //-------------

                            let elapsed = start_time.elapsed().as_micros() as f32 / 1000.;

                            if movement_mode {
                                let delta = current_mouse_pos - mouse_resting_position;

                                scene_renderer.on_update(
                                    &device,
                                    &queue,
                                    delta,
                                    &elapsed,
                                    &platform.context(),
                                );
                            }

                            if platform
                                .context()
                                .input(|i: &egui::InputState| i.key_pressed(egui::Key::F11))
                            {
                                show_ui = !show_ui;
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
        .unwrap();
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
    adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                required_limits: wgpu::Limits::downlevel_defaults()
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

fn generate_instance() -> Instance {
    let instance_desc: wgpu::InstanceDescriptor = InstanceDescriptor {
        backends: Backends::VULKAN,
        flags: InstanceFlags::default(),
        dx12_shader_compiler: Dx12Compiler::default(),
        gles_minor_version: Gles3MinorVersion::default(),
    };

    wgpu::Instance::new(instance_desc)
}

// ######################### compute ########################################

fn create_ui(
    device: &wgpu::Device,
    queue: &Queue,
    platform: &mut Platform,
    screne_renderer: &mut Renderer,
) -> FullOutput {
    platform.begin_frame();

    // important, create a egui context, do not use platform.conmtext()
    let egui_context = platform.context();

    let mut style = (*egui_context.style()).clone();
    style.visuals.override_text_color = Some(Color32::from_rgb(220, 220, 220));
    egui_context.set_style(style);

    let transparent_frame = Frame::none().fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200));

    let mut interacted = false;

    egui::SidePanel::right("side_panel")
        .resizable(false)
        .frame(transparent_frame)
        .show(&egui_context, |ui| {
            ui.set_max_width(100.0);

            ui.vertical_centered(|ui| {
                if ui
                    .checkbox(&mut screne_renderer.accumulate, "light accumulation")
                    .changed()
                {
                    interacted = true;
                };

                ui.label("light mode:");
                if ui
                    .add(
                        egui::Slider::new(&mut screne_renderer.light_mode, 0..=3)
                            .integer()
                            .show_value(false),
                    )
                    .changed()
                {
                    interacted = true;
                };

                let sky_color = &mut screne_renderer.scene.sky_color;

                ui.label("sky color:");
                if ui
                    .color_edit_button_rgb(sky_color)
                    .on_hover_text("color")
                    .changed()
                {
                    interacted = true;
                };

                // len - 1 because the last sphere is the floor sphere
                let sphere_count = screne_renderer.scene.spheres.len();
                for i in 0..sphere_count {
                    let floor_sphere = i == (sphere_count - 1);

                    let current_sphere = &mut screne_renderer.scene.spheres[i];
                    let num = i + 1;

                    if !floor_sphere {
                        ui.label(format!("sphere {num} values"));
                    } else {
                        ui.label("floor values".to_string());
                    }

                    // X Y Z sliders
                    if !floor_sphere {
                        let sphere_position = &mut current_sphere.position;

                        ui.horizontal(|ui| {
                            if ui
                                .add(
                                    DragValue::new(&mut sphere_position[0])
                                        .speed(0.1)
                                        .clamp_range(-100.0..=100.0)
                                        .prefix("X: "),
                                )
                                .changed()
                            {
                                interacted = true;
                            };

                            if ui
                                .add(
                                    DragValue::new(&mut sphere_position[1])
                                        .speed(0.1)
                                        .clamp_range(-100.0..=0.0)
                                        .prefix("Y: "),
                                )
                                .changed()
                            {
                                interacted = true;
                            };
                            if ui
                                .add(
                                    DragValue::new(&mut sphere_position[2])
                                        .speed(0.1)
                                        .clamp_range(-100.0..=100.0)
                                        .prefix("Z: "),
                                )
                                .changed()
                            {
                                interacted = true;
                            };
                        });
                    }

                    // sliders for radius, roughness and metallic
                    ui.vertical_centered_justified(|ui: &mut egui::Ui| {
                        let material_index = current_sphere.material_index as usize;
                        let current_material = &mut screne_renderer.scene.materials[material_index];

                        let sphere_color = &mut current_material.albedo;

                        let sphere_emission_color = &mut current_material.emission_color;

                        let emission_power = &mut current_material.emission_power;

                        ui.horizontal(|ui| {
                            if ui
                                .color_edit_button_rgb(sphere_color)
                                .on_hover_text("color")
                                .changed()
                            {
                                interacted = true;
                            };

                            if ui
                                .color_edit_button_rgb(sphere_emission_color)
                                .on_hover_text("emission")
                                .changed()
                            {
                                interacted = true;
                            };
                        });

                        if ui
                            .add(
                                DragValue::new(emission_power)
                                    .speed(0.2)
                                    .clamp_range(0.0..=200.0)
                                    .prefix("emission power: "),
                            )
                            .changed()
                        {
                            interacted = true;
                        };

                        if !floor_sphere {
                            let sphere_radius = &mut current_sphere.radius;

                            if ui
                                .add(
                                    DragValue::new(sphere_radius)
                                        .speed(0.01)
                                        .clamp_range(0.1..=30.0)
                                        .prefix("radius: "),
                                )
                                .changed()
                            {
                                interacted = true;
                            };
                        }

                        let sphere_roughness = &mut current_material.roughness;

                        if ui
                            .add(
                                DragValue::new(sphere_roughness)
                                    .speed(0.01)
                                    .clamp_range(0.0..=1.0)
                                    .prefix("roughness: "),
                            )
                            .changed()
                        {
                            interacted = true;
                        };

                        let sphere_metallic = &mut current_material.metallic;

                        if ui
                            .add(
                                DragValue::new(sphere_metallic)
                                    .speed(0.01)
                                    .clamp_range(0.0..=1.0)
                                    .prefix("metallic: "),
                            )
                            .changed()
                        {
                            interacted = true;
                        };
                    });

                    ui.add_space(15.0);
                }
            });
        });

    if interacted {
        screne_renderer.update_scene(device, queue)
    }

    egui_context.end_frame()
}
