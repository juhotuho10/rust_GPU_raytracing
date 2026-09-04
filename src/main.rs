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

use egui::{Color32, DragValue, Frame, FullOutput};

use wgpu::{
    Adapter, Backends, BindGroup, BlendState, Device, InstanceDescriptor, PipelineLayout, Queue,
    Surface, TextureFormat, TextureUsages, include_wgsl,
};

use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceEvent, ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{CursorGrabMode, Window, WindowId},
};

use egui_wgpu_backend::{RenderPass as EguiRenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};

use std::sync::Arc;
use std::time::{Duration, Instant};

pub const TRIANGLE_COUNT: u64 = 5552;
pub const SUBOBJECT_COUNT: u64 = 802;
pub const OBJECT_COUNT: u64 = 34;

pub const SPHERE_COUNT: u64 = 3;
pub const MATERIAL_COUNT: u64 = 19;

const SURFACE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

const FRAMETIME: Duration = Duration::from_millis(5);

const COMPUTETIME: Duration = Duration::from_micros(800);

const COMPUTATION_PER_FRAME: u32 = 5;

pub fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().expect("failed to make eventloop");

    let mut app = App::new();

    event_loop.run_app(&mut app).expect("Eventloop failed");

    // App fields drop in declaration order: gpu drops with the renderer, which owns
    // the device and queue, so GPU resources are released safely.
}

struct App {
    gpu: Option<Gpu>,
    movement_mode: bool,
    mouse_resting_position: egui::Pos2,
    mouse_delta: egui::Vec2,
    last_mouse_pos: egui::Pos2,
    show_ui: bool,
    compute_counter: u32,
    compute_per_second: u32,
    frame_timer: Timer,
    compute_timer: Timer,
    compute_per_second_timer: Timer,
}

// Everything that requires an existing window. Created once in `resumed`.
struct Gpu {
    window: Arc<Window>,
    surface: Surface<'static>,
    renderer: Renderer,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: BindGroup,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    surface_config: wgpu::SurfaceConfiguration,
    egui: EguiState,
}

struct EguiState {
    platform: Platform,
    screen_descriptor: ScreenDescriptor,
    rpass: EguiRenderPass,
}

// ##########################################################################################################################
// ########################################################## Timer #########################################################
// ##########################################################################################################################
struct Timer {
    period: Duration,
    last: Instant,
}

impl Timer {
    fn new(period: Duration) -> Self {
        Self {
            period,
            last: Instant::now(),
        }
    }

    fn ready(&mut self) -> bool {
        if self.last.elapsed() >= self.period {
            self.last = Instant::now();
            true
        } else {
            false
        }
    }
}

// ##########################################################################################################################
// ########################################################## App imp #######################################################
// ##########################################################################################################################

fn center_of(size: PhysicalSize<u32>) -> egui::Pos2 {
    egui::pos2(
        (size.width as f32 / 2.).round(),
        (size.height as f32 / 2.).round(),
    )
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_none() {
            let gpu = Gpu::new(event_loop);
            self.mouse_resting_position = center_of(gpu.window.inner_size());
            self.gpu = Some(gpu);
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event
            && self.movement_mode
        {
            self.mouse_delta += egui::vec2(dx as f32, dy as f32);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.egui.platform.handle_event(&event);
        }
        self.handle_window_event(event, event_loop);
    }

    // `about_to_wait` fires after every event batch, so redraws are requested here
    // instead of in each individual event arm.
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(gpu) = self.gpu.as_ref() {
            gpu.window.request_redraw();
        }
    }
}

impl App {
    fn new() -> Self {
        Self {
            gpu: None,
            movement_mode: false,
            mouse_resting_position: egui::pos2(0.0, 0.0),
            mouse_delta: egui::vec2(0.0, 0.0),
            last_mouse_pos: egui::pos2(0.0, 0.0),
            show_ui: true,
            compute_counter: 0,
            compute_per_second: 0,
            frame_timer: Timer::new(FRAMETIME),
            compute_timer: Timer::new(COMPUTETIME),
            compute_per_second_timer: Timer::new(Duration::from_secs(1)),
        }
    }

    fn handle_window_event(&mut self, event: WindowEvent, target: &ActiveEventLoop) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };
        let window = gpu.window.clone();

        match event {
            WindowEvent::Resized(new_size) => {
                gpu.resize(PhysicalSize::new(
                    new_size.width.max(1),
                    new_size.height.max(1),
                ));
                self.mouse_resting_position = center_of(gpu.window.inner_size());
            }

            WindowEvent::CloseRequested => {
                // Exit the application
                gpu.renderer
                    .device
                    .poll(wgpu::PollType::wait_indefinitely())
                    .expect("device poll failed");
                target.exit();
            }

            WindowEvent::MouseInput {
                state,
                button: MouseButton::Right,
                ..
            } => match state {
                ElementState::Pressed => {
                    let grabbed = window.set_cursor_grab(CursorGrabMode::Confined);

                    let possible_pos = gpu
                        .egui
                        .platform
                        .context()
                        .input(|i: &egui::InputState| i.pointer.hover_pos());

                    match (grabbed, possible_pos) {
                        (Ok(_), Some(pos)) => {
                            self.movement_mode = true;
                            window.set_cursor_visible(false);
                            self.last_mouse_pos = pos;
                            self.mouse_delta = egui::vec2(0.0, 0.0);
                        }
                        (Err(error), _) => println!("cound not grab the cursor, {}", error),
                        (_, _) => println!("could not find cursor position"),
                    }
                }
                ElementState::Released => {
                    let grab_release = window.set_cursor_grab(CursorGrabMode::None);
                    window.set_cursor_visible(true);

                    let pos_set = window.set_cursor_position(PhysicalPosition::new(
                        self.last_mouse_pos.x as u32,
                        self.last_mouse_pos.y as u32,
                    ));

                    match (grab_release, pos_set) {
                        (Ok(_), _) => self.movement_mode = false,
                        (Err(error), _) => println!("could not release cursor, {}", error),
                    }
                }
            },

            WindowEvent::RedrawRequested => {
                self.redraw();
            }

            _ => {}
        }
    }

    fn redraw(&mut self) {
        let Some(gpu) = self.gpu.as_mut() else {
            return;
        };

        if self.compute_timer.ready() {
            self.compute_counter += COMPUTATION_PER_FRAME;
            gpu.renderer
                .compute_frame(&gpu.compute_pipeline, &gpu.compute_bind_group);
        }

        if !self.frame_timer.ready() {
            return;
        }

        if self.movement_mode {
            let _ = gpu.window.set_cursor_position(PhysicalPosition::new(
                self.mouse_resting_position.x,
                self.mouse_resting_position.y,
            ));
        }

        let mut encoder =
            gpu.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Encoder"),
                });

        let frame = gpu
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        setup_renderpass(&mut encoder, &view, &gpu.render_pipeline, &gpu.bind_group);

        if self.compute_per_second_timer.ready() {
            self.compute_per_second = self.compute_counter;
            self.compute_counter = 0;
        }

        let full_output = create_ui(
            &mut gpu.egui.platform,
            &mut gpu.renderer,
            &self.compute_per_second,
        );

        let paint_jobs = gpu
            .egui
            .platform
            .context()
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        if self.show_ui {
            let rpass = &mut gpu.egui.rpass;
            let screen = &gpu.egui.screen_descriptor;

            rpass
                .add_textures(
                    &gpu.renderer.device,
                    &gpu.renderer.queue,
                    &full_output.textures_delta,
                )
                .expect("couldnt add textures");
            rpass.update_buffers(
                &gpu.renderer.device,
                &gpu.renderer.queue,
                &paint_jobs,
                screen,
            );
            rpass
                .execute(&mut encoder, &view, &paint_jobs, screen, None)
                .expect("egui render pass failed");
        }

        gpu.renderer.queue.submit(Some(encoder.finish()));
        frame.present();

        gpu.egui
            .rpass
            .remove_textures(full_output.textures_delta)
            .expect("textures could not be removed");

        if self.movement_mode {
            let delta = std::mem::take(&mut self.mouse_delta);
            gpu.renderer.on_update(delta, &gpu.egui.platform.context());
        }

        if gpu
            .egui
            .platform
            .context()
            .input(|i: &egui::InputState| i.key_pressed(egui::Key::F11))
        {
            self.show_ui = !self.show_ui;
        }
    }
}

// ##########################################################################################################################
// ####################################################### GPU impl #########################################################
// ##########################################################################################################################

impl Gpu {
    fn new(target: &ActiveEventLoop) -> Self {
        let instance = wgpu::Instance::new(&InstanceDescriptor {
            backends: Backends::VULKAN,
            ..Default::default()
        });

        // window width is set at 1600, because GPU buffer requires n * 256 bytes (n * 64 pixels * 4*u8 colors ) for every horisontal row,
        // changing it to not be a multiple of 64 requires implementing buffer values when getting colors from the GPU
        let window = Arc::new(
            target
                .create_window(
                    Window::default_attributes()
                        .with_inner_size(PhysicalSize::new(1600, 900))
                        .with_resizable(false),
                )
                .expect("failed to make window"),
        );

        let surface: Surface<'static> = instance
            .create_surface(window.clone())
            .expect("failed to make a surface");

        let adapter = pollster::block_on(create_adapter(&instance, &surface));
        // Create the logical device and command queue
        let (device, queue) = pollster::block_on(generate_device_and_queue(&adapter));

        let size = window.inner_size();
        let camera = Camera::new(size.width, size.height);

        let scene: renderer::RenderScene = define_render_scene();
        // ################################################################################

        let triangle_count: usize = scene
            .objects
            .iter()
            .map(|obj: &SceneObject| obj.object_triangles.len())
            .sum();
        let sub_object_count: usize = scene
            .objects
            .iter()
            .map(|obj: &SceneObject| obj.sub_object_info.len())
            .sum();

        println!(
            "the following numbers should be the same in the compute shader for the buffer sizes"
        );
        dbg!(triangle_count);
        dbg!(sub_object_count);
        dbg!(scene.objects.len());
        dbg!(scene.spheres.len());
        dbg!(scene.materials.len());

        assert_eq!(triangle_count, TRIANGLE_COUNT as usize);
        assert_eq!(sub_object_count, SUBOBJECT_COUNT as usize);
        assert_eq!(scene.objects.len(), OBJECT_COUNT as usize);
        assert_eq!(scene.spheres.len(), SPHERE_COUNT as usize);
        assert_eq!(scene.materials.len(), MATERIAL_COUNT as usize);

        // ################################################################################

        let params = Params {
            screen_width: size.width,
            screen_height: size.height,
            accumulation_index: 1,
            accumulate: 1,
            sphere_count: scene.spheres.len() as u32,
            object_count: scene.objects.len() as u32,
            compute_per_frame: COMPUTATION_PER_FRAME,
            texture_width: scene.texture_size[0],
            texture_height: scene.texture_size[1],
            textue_count: scene.image_textures.len() as u32,
            env_map_width: scene.env_map_size[0],
            env_map_height: scene.env_map_size[1],
        };

        let (renderer, compute_bindgroup_layout, compute_bind_group) =
            Renderer::new(camera, scene, device, queue, size, params);

        // The renderer owns the device and queue; everything below uses those.

        // ################################ GPU COMPUTE PIPELINE #########################################

        let compute_shader_code = include_str!("compute_shader.wgsl")
            .replace("TRIANGLE_COUNT_PLACEHOLDER", &TRIANGLE_COUNT.to_string())
            .replace("SUBOBJECT_COUNT_PLACEHOLDER", &SUBOBJECT_COUNT.to_string())
            .replace("OBJECT_COUNT_PLACEHOLDER", &OBJECT_COUNT.to_string())
            .replace("SPHERE_COUNT_PLACEHOLDER", &SPHERE_COUNT.to_string())
            .replace("MATERIAL_COUNT_PLACEHOLDER", &MATERIAL_COUNT.to_string());

        let compute_module = renderer
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("compute_shader.wgsl"),
                source: wgpu::ShaderSource::Wgsl(compute_shader_code.into()),
            });

        let compute_pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Compute Pipeline Layout"),
                    bind_group_layouts: &[&compute_bindgroup_layout],
                    immediate_size: 0,
                });

        let compute_pipeline =
            renderer
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Compute Pipeline"),
                    layout: Some(&compute_pipeline_layout),
                    module: &compute_module,
                    entry_point: Some("main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    cache: None,
                });

        // #####################################################################################
        // ################################ RENDER PIPELINE #########################################
        // #####################################################################################

        let sampler = generate_sampler(&renderer.device);
        let (bind_group_layout, bind_group) =
            create_device_bindgroup(&renderer.device, &renderer.output_texture_view(), &sampler);

        let pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&bind_group_layout],
                    immediate_size: 0,
                });

        let render_pipeline =
            create_render_pipeline(&renderer.device, &pipeline_layout, SURFACE_FORMAT);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: SURFACE_FORMAT,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Immediate,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![SURFACE_FORMAT],
        };

        surface.configure(&renderer.device, &surface_config);

        let scale_factor = window.scale_factor();
        let egui = EguiState {
            platform: Platform::new(PlatformDescriptor {
                physical_width: size.width,
                physical_height: size.height,
                scale_factor,
                font_definitions: Default::default(),
                style: Default::default(),
            }),
            screen_descriptor: ScreenDescriptor {
                physical_width: size.width,
                physical_height: size.height,
                scale_factor: scale_factor as f32,
            },
            rpass: EguiRenderPass::new(&renderer.device, surface_config.format, 1),
        };

        Self {
            window,
            surface,
            renderer,
            compute_pipeline,
            compute_bind_group,
            sampler,
            bind_group_layout,
            bind_group,
            render_pipeline,
            surface_config,
            egui,
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;

        self.egui.screen_descriptor.physical_width = size.width;
        self.egui.screen_descriptor.physical_height = size.height;

        self.compute_bind_group = self.renderer.on_resize(&size);

        let (bind_group_layout, bind_group) = create_device_bindgroup(
            &self.renderer.device,
            &self.renderer.output_texture_view(),
            &self.sampler,
        );
        self.bind_group_layout = bind_group_layout;
        self.bind_group = bind_group;

        self.surface
            .configure(&self.renderer.device, &self.surface_config);
    }
}

fn create_device_bindgroup(
    device: &wgpu::Device,
    texture_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> (wgpu::BindGroupLayout, BindGroup) {
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
                resource: wgpu::BindingResource::TextureView(texture_view),
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
        multiview_mask: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            depth_slice: None,
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
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: swapchain_format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
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
        .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
            required_limits: adapter_limits,
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
        })
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
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    })
}

// ######################### UI CREATION ########################################

// simple macro for making the UI more compact
macro_rules! create_drag_value {
    ($ui:expr, $value:expr, $speed:expr, $range:expr, $prefix:expr) => {{
        $ui.add(
            DragValue::new($value)
                .speed($speed)
                .range($range)
                .prefix($prefix),
        )
        .changed()
    }};
}

fn create_ui(
    platform: &mut Platform,
    screne_renderer: &mut Renderer,
    compute_per_second: &u32,
) -> FullOutput {
    platform.begin_pass();

    // important, create a egui context, do not use platform.conmtext()
    let egui_context = platform.context();

    let mut style = (*egui_context.style()).clone();
    style.visuals.override_text_color = Some(Color32::from_rgb(200, 200, 200));
    egui_context.set_style(style);

    let transparent_frame = Frame::new().fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200));

    let mut interacted = false;

    egui::SidePanel::right("side_panel")
        .resizable(false)
        .frame(transparent_frame)
        .show(&egui_context, |ui| {
            ui.set_max_width(180.0);

            ui.label(format!("fps: {}", compute_per_second));

            ui.vertical_centered(|ui| {
                let sky_color = &mut screne_renderer.scene.environment_map.color;
                if let Some(sky_color) = sky_color {
                    ui.label("sky color:");
                    if ui
                        .color_edit_button_rgb(sky_color)
                        .on_hover_text("color")
                        .changed()
                    {
                        interacted = true;
                    };
                }

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

    egui_context.end_pass()
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

        if let Some(color) = color
            && ui
                .color_edit_button_rgb(color)
                .on_hover_text("color")
                .changed()
        {
            *interacted = true;
        };

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
