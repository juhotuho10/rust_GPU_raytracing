mod Scene;
mod camera;
mod renderer;

use camera::Camera;
use Scene::{Material, RenderScene, Sphere};

use egui::{pos2, Color32, DragValue, Frame, FullOutput};

use renderer::Renderer;
use std::{borrow::Cow, time};
use wgpu::{
    include_wgsl, util::DeviceExt, Adapter, Backends, BindGroup, Buffer, Device, Dx12Compiler,
    Gles3MinorVersion, Instance, InstanceDescriptor, InstanceFlags, PipelineCompilationOptions,
    PipelineLayout, Queue, ShaderModule, Surface, SurfaceConfiguration, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages,
};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, Event, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window},
};

use egui_wgpu_backend::{RenderPass as EguiRenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use glam::{vec3, vec3a, Vec3, Vec3A};

use std::time::Instant;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    width: u32,
}

pub fn main() {
    let event_loop = EventLoop::new().unwrap();

    let builder = winit::window::WindowBuilder::new();

    let window_size = PhysicalSize::new(1400, 700);

    let window = builder
        .with_inner_size(window_size)
        .build(&event_loop)
        .unwrap();

    env_logger::init();
    pollster::block_on(run(event_loop, window));
}

fn define_scene() -> RenderScene {
    let shiny_green = Material {
        albedo: vec3a(0.1, 0.8, 0.4),
        roughness: 0.3,
        metallic: 1.0,
        emission_color: vec3a(0.1, 0.8, 0.4),
        emission_power: 0.0,
    };

    let rough_blue = Material {
        albedo: vec3a(0.3, 0.2, 0.8),
        roughness: 0.7,
        metallic: 0.5,
        emission_color: vec3a(0.3, 0.2, 0.8),
        emission_power: 0.0,
    };

    let glossy_pink = Material {
        albedo: vec3a(1.0, 0.1, 1.0),
        roughness: 0.4,
        metallic: 0.8,
        emission_color: vec3a(1.0, 0.1, 1.0),
        emission_power: 0.0,
    };

    let shiny_orange = Material {
        albedo: vec3a(1.0, 0.7, 0.0),
        roughness: 0.7,
        metallic: 0.7,
        emission_color: vec3a(1.0, 0.7, 0.0),
        emission_power: 10.0,
    };

    let sphere_a: Sphere = Sphere {
        position: vec3a(0., -1., 0.),
        radius: 0.5,

        material_index: 2,
    };

    let sphere_b: Sphere = Sphere {
        position: vec3a(-3., -2.0, 3.),
        radius: 2.0,

        material_index: 0,
    };

    let shiny_sphere: Sphere = Sphere {
        position: vec3a(3., -15.0, -5.),
        radius: 7.0,

        material_index: 3,
    };

    // sphere to act as a floor
    let floor: Sphere = Sphere {
        position: vec3a(0., 500., 0.),
        radius: 500.,

        material_index: 1,
    };

    let scene: RenderScene = RenderScene {
        spheres: vec![sphere_a, sphere_b, shiny_sphere, floor],
        materials: vec![shiny_green, rough_blue, glossy_pink, shiny_orange],
        sky_color: vec3a(0., 0.04, 0.1),
    };

    scene
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

    let scene = define_scene();

    let mut scene_renderer = Renderer::new(camera, scene);

    let mut last_mouse_pos: egui::Pos2 = pos2(0., 0.);

    let instance = generate_instance();

    let surface: Surface = instance.create_surface(&window).unwrap();
    let adapter = create_adapter(&instance, &surface).await;
    // Create the logical device and command queue
    let (device, queue) = generate_device_and_queue(&adapter).await;

    // ################################ GPU DATA PIPELINE #########################################

    let mut input_data = generate_data(&size);
    let (mut input_buffer, mut output_buffer, mut shader_params) =
        create_buffers(&device, &input_data, &size);

    let mut bytes_per_row = calculate_bytes_per_row(&size);

    let mut compute_bindgroup_layout = create_compute_bindgroup_layout(&device);

    let mut compute_bind_group = create_compute_bindgroup(
        &device,
        &compute_bindgroup_layout,
        &input_buffer,
        &output_buffer,
        &shader_params,
    );

    // #####################################################################################

    // ################################ GPU COMPUTE PIPELINE #########################################

    let shader_module = device.create_shader_module(include_wgsl!("shader.wgsl"));

    let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Compute Pipeline Layout"),
        bind_group_layouts: &[&compute_bindgroup_layout],
        push_constant_ranges: &[],
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute Pipeline"),
        layout: Some(&compute_pipeline_layout),
        module: &shader_module,
        entry_point: "main",
        compilation_options: wgpu::PipelineCompilationOptions::default(),
    });

    // #####################################################################################

    let mut texture = create_texture(&device, size);

    let render_bind_group_layout: wgpu::BindGroupLayout = generate_bind_group_layout(&device);

    let sampler: wgpu::Sampler = generate_sampler(&device);

    let mut render_bind_group =
        create_device_bindgroup(&device, &render_bind_group_layout, &texture, &sampler);

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&render_bind_group_layout],
        push_constant_ranges: &[],
    });

    let render_pipeline = create_render_pipeline(
        &device,
        &render_pipeline_layout,
        texture.format(),
        &shader_module,
    );

    let mut surface_config = wgpu::SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: texture.format(),
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
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
            let _ = (&instance, &render_pipeline_layout);

            match event {
                Event::DeviceEvent { .. } => {
                    window.request_redraw();
                }
                Event::WindowEvent { event, .. } => {
                    let start_time = Instant::now();

                    match event {
                        WindowEvent::CursorMoved {
                            device_id: _,
                            position: _,
                        } => {
                            window.request_redraw();
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

                            scene_renderer.on_resize(size.width, size.height);

                            texture = create_texture(&device, size);

                            /*bind_group = create_device_bindgroup(
                                &device,
                                &bind_group_layout,
                                &texture,
                                &sampler,
                            );*/

                            // ###################### COMPUTE RESIZING ##################################
                            input_data = generate_data(&size);
                            (input_buffer, output_buffer, shader_params) =
                                create_buffers(&device, &input_data, &size);

                            bytes_per_row = calculate_bytes_per_row(&size);

                            compute_bindgroup_layout = create_compute_bindgroup_layout(&device);

                            compute_bind_group = create_compute_bindgroup(
                                &device,
                                &compute_bindgroup_layout,
                                &input_buffer,
                                &output_buffer,
                                &shader_params,
                            );
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

                                window.request_redraw();
                            }
                        },

                        WindowEvent::RedrawRequested => {
                            // Create command encoder and dispatch compute pass
                            let mut encoder =
                                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("Compute Encoder"),
                                });

                            {
                                let mut compute_pass =
                                    encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                                        label: Some("Compute Pass"),
                                        timestamp_writes: None,
                                    });
                                compute_pass.set_pipeline(&compute_pipeline);
                                compute_pass.set_bind_group(0, &compute_bind_group, &[]);
                                compute_pass.dispatch_workgroups(
                                    size.width / 8,
                                    size.height / 8,
                                    1,
                                );
                            }

                            queue.submit(Some(encoder.finish()));

                            // ####################### move compute results into texture ##################################
                            let mut encoder =
                                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("Render Encoder"),
                                });

                            let texture_extent = wgpu::Extent3d {
                                width: size.width,
                                height: size.height,
                                depth_or_array_layers: 1,
                            };

                            // Copy the output buffer to the texture
                            encoder.copy_buffer_to_texture(
                                wgpu::ImageCopyBuffer {
                                    buffer: &output_buffer,
                                    layout: wgpu::ImageDataLayout {
                                        offset: 0,
                                        bytes_per_row: Some(bytes_per_row), // 4 bytes per pixel
                                        rows_per_image: Some(size.height),
                                    },
                                },
                                wgpu::ImageCopyTexture {
                                    texture: &texture,
                                    mip_level: 0,
                                    origin: wgpu::Origin3d::ZERO,
                                    aspect: wgpu::TextureAspect::All,
                                },
                                texture_extent,
                            );

                            queue.submit(Some(encoder.finish()));

                            let frame: wgpu::SurfaceTexture = surface
                                .get_current_texture()
                                .expect("Failed to acquire next swap chain texture");

                            let view: wgpu::TextureView = frame
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            let mut render_encoder: wgpu::CommandEncoder = device
                                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: None,
                                });

                            {
                                let mut render_pass =
                                    render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        label: Some("Render Pass"),
                                        color_attachments: &[Some(
                                            wgpu::RenderPassColorAttachment {
                                                view: &view,
                                                resolve_target: None,
                                                ops: wgpu::Operations {
                                                    load: wgpu::LoadOp::Load,
                                                    store: wgpu::StoreOp::Store,
                                                },
                                            },
                                        )],
                                        depth_stencil_attachment: None,
                                        timestamp_writes: None,
                                        occlusion_query_set: None,
                                    });

                                render_pass.set_pipeline(&render_pipeline);
                                render_pass.set_bind_group(0, &render_bind_group, &[]);
                                render_pass.draw(0..6, 0..1);
                            }

                            queue.submit(Some(render_encoder.finish()));
                            frame.present();

                            /*if movement_mode {
                                window
                                    .set_cursor_position(PhysicalPosition::new(
                                        mouse_resting_position.x,
                                        mouse_resting_position.y,
                                    ))
                                    .expect("couldn't set cursor pos");
                            }
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

                            let pixel_colors = scene_renderer.generate_pixels();

                            update_render_queue(&queue, &texture, &size, &pixel_colors);

                            setup_renderpass(&mut encoder, &view, &render_pipeline, &bind_group);

                            let full_output = create_ui(&mut platform, &mut scene_renderer);

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

                            //egui_rpass
                            //    .remove_textures(full_output.textures_delta)
                            //    .expect("textures removed");

                            //-------------

                            let elapsed = start_time.elapsed().as_micros() as f32 / 1000.;

                            if movement_mode {
                                current_mouse_pos = platform
                                    .context()
                                    .input(|i: &egui::InputState| i.pointer.hover_pos())
                                    .unwrap();

                                let delta = current_mouse_pos - mouse_resting_position;

                                scene_renderer.on_update(delta, &elapsed, &platform.context());
                            }

                            if platform
                                .context()
                                .input(|i: &egui::InputState| i.key_pressed(egui::Key::F11))
                            {
                                show_ui = !show_ui;
                            }*/
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
        usage: TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_DST
            | wgpu::TextureUsages::RENDER_ATTACHMENT,

        view_formats: &[],
    })
}

fn create_device_bindgroup(
    device: &wgpu::Device,
    render_bind_group_layout: &wgpu::BindGroupLayout,
    texture: &wgpu::Texture,
    sampler: &wgpu::Sampler,
) -> BindGroup {
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: render_bind_group_layout,
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
    render_pipeline_layout: &PipelineLayout,
    swapchain_format: TextureFormat,
    fragment_shader_module: &ShaderModule,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: fragment_shader_module,
            entry_point: "vs_main",
            buffers: &[],
            compilation_options: PipelineCompilationOptions {
                constants: &Default::default(),
                zero_initialize_workgroup_memory: false,
            },
        },
        fragment: Some(wgpu::FragmentState {
            module: fragment_shader_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: swapchain_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: PipelineCompilationOptions {
                constants: &Default::default(), // No special constants, use default
                zero_initialize_workgroup_memory: false, // Default value
            },
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
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

fn generate_data(size: &winit::dpi::PhysicalSize<u32>) -> Vec<[f32; 3]> {
    let mut data_vec: Vec<[f32; 3]> = vec![];

    for y in 0..size.height {
        for x in 0..size.width {
            data_vec.push([x as f32, y as f32, 1.0])
        }
    }

    data_vec
}

fn create_buffers(
    device: &wgpu::Device,
    input_data: &[[f32; 3]],
    size: &winit::dpi::PhysicalSize<u32>,
) -> (Buffer, Buffer, Buffer) {
    let buffer_size = std::mem::size_of_val(input_data) as wgpu::BufferAddress;

    let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Input Buffer"),
        contents: bytemuck::cast_slice(input_data),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: buffer_size / 3 * 4, // vec3 to vec4
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // Create uniform buffer
    let params = Params { width: size.width };

    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Params Buffer"),
        contents: bytemuck::cast_slice(&[params]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    (input_buffer, output_buffer, params_buffer)
}

fn create_compute_bindgroup_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
        label: None,
    })
}

fn create_compute_bindgroup(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    input_buffer: &Buffer,
    output_buffer: &Buffer,
    shader_params: &Buffer,
) -> BindGroup {
    let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: shader_params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: input_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: output_buffer.as_entire_binding(),
            },
        ],
        label: None,
    });

    compute_bind_group
}

fn calculate_bytes_per_row(size: &winit::dpi::PhysicalSize<u32>) -> u32 {
    // the gpu buffer has to be 256 * n bytes per row

    let bytes_per_pixel = 4; // RGBA8Unorm = 4 bytes per pixel

    let value = size.width * bytes_per_pixel;
    let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

    // bytes per row
    (value + alignment - 1) & !(alignment - 1)
}

fn create_ui(platform: &mut Platform, screne_renderer: &mut Renderer) -> FullOutput {
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
                ui.checkbox(&mut screne_renderer.accumulate, "light accumulation");

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
                    .color_edit_button_rgb(sky_color.as_mut())
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
                                    DragValue::new(&mut sphere_position.x)
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
                                    DragValue::new(&mut sphere_position.y)
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
                                    DragValue::new(&mut sphere_position.z)
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
                        let material_index = current_sphere.material_index;
                        let current_material = &mut screne_renderer.scene.materials[material_index];

                        let sphere_color = &mut current_material.albedo;

                        let sphere_emission_color = &mut current_material.emission_color;

                        let emission_power = &mut current_material.emission_power;

                        ui.horizontal(|ui| {
                            if ui
                                .color_edit_button_rgb(sphere_color.as_mut())
                                .on_hover_text("color")
                                .changed()
                            {
                                interacted = true;
                            };

                            if ui
                                .color_edit_button_rgb(sphere_emission_color.as_mut())
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
        screne_renderer.reset_accumulation()
    }

    egui_context.end_frame()
}
