use egui::{Frame, FullOutput};
use rand::Rng;
use rayon::prelude::*;
use std::borrow::Cow;
use wgpu::{
    Adapter, BindGroup, Device, PipelineLayout, Queue, Surface, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages,
};
use winit::{
    dpi::PhysicalPosition,
    event::{self, ElementState, Event, MouseButton, WindowEvent},
    event_loop::EventLoop,
    window::{CursorGrabMode, Window},
};

use egui_wgpu_backend::{RenderPass as EguiRenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};

pub fn main() {
    let event_loop = EventLoop::new().unwrap();

    let builder = winit::window::WindowBuilder::new();

    let window = builder.build(&event_loop).unwrap();

    env_logger::init();
    pollster::block_on(run(event_loop, window));
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    let mut rng = rand::thread_rng();

    let mut right_click_pressed = false;

    let mut size = window.inner_size();
    size.width = size.width.max(1);
    size.height = size.height.max(1);

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

    //// Modify egui styles for transparency
    //let mut style: egui::Style = (*platform.context().style()).clone();
    //style.visuals.widgets.noninteractive.bg_fill =
    //    egui::Color32::from_rgba_premultiplied(27, 27, 27, 25); // Slight grey, low alpha
    //style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgba_premultiplied(27, 27, 27, 25); // Apply same for inactive widgets if needed
    //style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgba_premultiplied(27, 27, 27, 50); // Darker when hovered
    //style.visuals.widgets.active.bg_fill = egui::Color32::from_rgba_premultiplied(27, 27, 27, 75); // Even darker when active
    //platform.context().set_style(style);

    let mut egui_rpass = EguiRenderPass::new(&device, config.format, 1);

    /* ##############################################3################################# */

    event_loop
        .run(move |event, target| {
            // Have the closure take ownership of the resources.
            // `event_loop.run` never returns, therefore we must do this to ensure
            // the resources are properly cleaned up.

            let _ = (&instance, &pipeline_layout);

            platform.handle_event(&event);

            window.request_redraw();

            if let Event::WindowEvent {
                window_id: _,
                event,
            } = event
            {
                match event {
                    WindowEvent::Resized(new_size) => {
                        // Reconfigure the surface with the new size

                        size.width = new_size.width.max(1);
                        size.height = new_size.height.max(1);

                        config.width = size.width;
                        config.height = size.height;

                        texture = create_texture(&device, size);

                        bind_group = create_device_bindgroup(
                            &device,
                            &bind_group_layout,
                            &texture,
                            &sampler,
                        );

                        screen_descriptor.physical_height = size.height;
                        screen_descriptor.physical_width = size.width;

                        surface.configure(&device, &config);
                        // On macos the window needs to be redrawn manually after resizing
                        window.request_redraw();
                    }

                    WindowEvent::RedrawRequested => {
                        let pixel_colors = generate_pixels(&device, &size, &mut rng);

                        update_render_queue(&queue, &texture, &size, &pixel_colors);

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

                        add_renderpass_to_encode(
                            &mut encoder,
                            &view,
                            &render_pipeline,
                            &bind_group,
                        );

                        let full_output = create_ui(window, &mut platform);

                        let paint_jobs = platform
                            .context()
                            .tessellate(full_output.shapes, full_output.pixels_per_point);

                        // ######### Adding egui renderpass to the encoder ###########
                        egui_rpass
                            .add_textures(&device, &queue, &full_output.textures_delta)
                            .expect("couldnt add textures");

                        egui_rpass.update_buffers(&device, &queue, &paint_jobs, &screen_descriptor);

                        egui_rpass
                            .execute(&mut encoder, &view, &paint_jobs, &screen_descriptor, None)
                            .unwrap();

                        // ######### rendering the queue ###########
                        queue.submit(Some(encoder.finish()));
                        frame.present();
                    }

                    WindowEvent::CloseRequested => target.exit(),

                    WindowEvent::MouseInput {
                        state,
                        button: MouseButton::Right,
                        device_id: _,
                    } => match state {
                        ElementState::Pressed => {
                            right_click_pressed = true;
                            window
                                .set_cursor_grab(CursorGrabMode::Confined)
                                .expect("couldn't confine cursor");

                            println!("cursor grabbed");
                        }
                        ElementState::Released => {
                            right_click_pressed = false;
                            window
                                .set_cursor_grab(CursorGrabMode::None)
                                .expect("Failed to release cursor");
                            window.set_cursor_visible(true);
                            println!("cursor released");
                        }
                    },
                    _ => {}
                };
            }

            if right_click_pressed {
                window
                    .set_cursor_position(PhysicalPosition::new(400, 200))
                    .expect("couldn't set cursor pos");
                window.set_cursor_visible(false);
            }
        })
        .unwrap();
}

#[allow(unused_variables)]
fn generate_pixels(
    device: &wgpu::Device,
    size: &winit::dpi::PhysicalSize<u32>,
    rng: &mut rand::rngs::ThreadRng,
) -> Vec<u8> {
    let mut pixel_colors: Vec<u8> = Vec::with_capacity((size.width * size.height * 4) as usize);
    for y in 0..size.height {
        for x in 0..size.width {
            let color: [u8; 4] = if rng.gen_bool(0.5) {
                [255, 255, 255, 255]
            } else {
                [0, 0, 0, 255]
            };
            pixel_colors.extend_from_slice(&color);
        }
    }
    pixel_colors
}

//fn generate_pixels(
//    device: &wgpu::Device,
//    size: winit::dpi::PhysicalSize<u32>,
//    rng: &mut rand::rngs::ThreadRng,
//) -> Vec<u8> {
//    // Generate a seed from the original RNG
//    let seed: [u8; 32] = rng.gen();
//
//    let pixel_colors: Vec<u8> = (0..size.height)
//        .into_par_iter()
//        .flat_map_iter(|y| {
//            // Derive a row-specific seed to ensure deterministic but varied results per row
//            let mut row_seed = seed;
//            row_seed[0] = row_seed[0].wrapping_add(y as u8); // Simple mutation of the seed based on row index
//
//            let mut local_rng = SmallRng::from_seed(row_seed);
//            let mut pixel_colors: Vec<u8> = Vec::with_capacity((size.width * 4) as usize);
//
//            for x in 0..size.width {
//                let color: [u8; 4] = if local_rng.gen_bool(0.5) {
//                    [255, 255, 255, 255]
//                } else {
//                    [0, 0, 0, 255]
//                };
//                pixel_colors.extend_from_slice(&color);
//            }
//            pixel_colors.into_iter()
//        })
//        .collect();
//
//    pixel_colors
//}

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

fn add_renderpass_to_encode(
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

fn create_ui(window: &Window, platform: &mut Platform) -> FullOutput {
    platform.begin_frame();

    let transparent_frame = Frame::none().fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200));
    egui::SidePanel::right("side_panel")
        .resizable(false)
        .frame(transparent_frame)
        .show(&platform.context(), |ui| {
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

    platform.end_frame(Some(window))
}
