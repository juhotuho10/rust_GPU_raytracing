#[allow(dead_code, unused)]
mod buffers;
mod camera;
mod define_scene;
mod image_texture;
mod renderer;
mod triangle_object;

use buffers::Params;
use camera::Camera;

use renderer::Renderer;

use define_scene::define_render_scene;
use triangle_object::SceneObject;

use egui::{Color32, DragValue, Frame, Stroke};

use wgpu::{
    Adapter, BindGroup, BlendState, Device, InstanceDescriptor, PipelineLayout, Queue,
    ShaderRuntimeChecks, Surface, TextureFormat, TextureUsages, include_wgsl,
};

use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceEvent, ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{CursorGrabMode, Window, WindowId},
};

use egui_wgpu::ScreenDescriptor;

use std::sync::Arc;
use std::time::{Duration, Instant};

const SURFACE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

const FRAMETIME: Duration = Duration::from_millis(5);

const COMPUTETIME: Duration = Duration::from_micros(800);

const COMPUTATION_PER_FRAME: u32 = 10;

pub fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().expect("failed to make eventloop");

    let mut app = App::new();

    event_loop.run_app(&mut app).expect("Eventloop failed")
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
struct Gpu {
    window: Arc<Window>,
    surface: Surface<'static>,
    renderer: Renderer,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: BindGroup,
    sampler: wgpu::Sampler,
    bind_group: BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    surface_config: wgpu::SurfaceConfiguration,
    egui: EguiState,
}

struct EguiState {
    winit: egui_winit::State,
    renderer: egui_wgpu::Renderer,
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
            // The repaint flag is ignored because a redraw is requested after every event batch.
            let _ = gpu.egui.winit.on_window_event(&gpu.window, &event);
        }
        self.handle_window_event(event, event_loop);
    }

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
            mouse_resting_position: egui::Pos2::ZERO,
            mouse_delta: egui::Vec2::ZERO,
            last_mouse_pos: egui::Pos2::ZERO,
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
                    let grabbed = gpu.window.set_cursor_grab(CursorGrabMode::Confined);
                    let hover_pos = gpu
                        .egui
                        .winit
                        .egui_ctx()
                        .input(|i: &egui::InputState| i.pointer.hover_pos());

                    match (grabbed, hover_pos) {
                        (Ok(_), Some(pos)) => {
                            self.movement_mode = true;
                            gpu.window.set_cursor_visible(false);
                            self.last_mouse_pos = pos;
                            self.mouse_delta = egui::Vec2::ZERO;
                        }
                        (Err(error), _) => println!("could not grab the cursor, {}", error),
                        (_, None) => println!("could not find cursor position"),
                    }
                }
                ElementState::Released => {
                    if let Err(error) = gpu.window.set_cursor_grab(CursorGrabMode::None) {
                        println!("could not release cursor, {}", error);
                    }
                    gpu.window.set_cursor_visible(true);
                    let _ = gpu.window.set_cursor_position(PhysicalPosition::new(
                        self.last_mouse_pos.x as u32,
                        self.last_mouse_pos.y as u32,
                    ));
                    self.movement_mode = false;
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

        let wgpu::CurrentSurfaceTexture::Success(frame) = gpu.surface.get_current_texture() else {
            // failed to get the frame, returning without a render
            return;
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        if self.compute_per_second_timer.ready() {
            self.compute_per_second = self.compute_counter;
            self.compute_counter = 0;
        }

        let raw_input = gpu.egui.winit.take_egui_input(gpu.window.as_ref());
        let mut full_output = gpu.egui.winit.egui_ctx().run_ui(raw_input, |ui| {
            create_ui(ui, &mut gpu.renderer, &self.compute_per_second)
        });
        gpu.egui.winit.handle_platform_output(
            gpu.window.as_ref(),
            std::mem::take(&mut full_output.platform_output),
        );

        let paint_jobs = gpu
            .egui
            .winit
            .egui_ctx()
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        let screen = ScreenDescriptor {
            size_in_pixels: [gpu.surface_config.width, gpu.surface_config.height],
            pixels_per_point: gpu.window.scale_factor() as f32,
        };

        // Upload egui font/image textures before they are used in the render pass.
        for (id, image_delta) in &full_output.textures_delta.set {
            gpu.egui.renderer.update_texture(
                &gpu.renderer.device,
                &gpu.renderer.queue,
                *id,
                &image_delta[0],
            );
        }

        gpu.egui.renderer.update_buffers(
            &gpu.renderer.device,
            &gpu.renderer.queue,
            &mut encoder,
            &paint_jobs,
            &screen,
        );

        {
            let mut render_pass =
                setup_renderpass(&mut encoder, &view, &gpu.render_pipeline, &gpu.bind_group);

            if self.show_ui {
                gpu.egui
                    .renderer
                    .render(&mut render_pass, &paint_jobs, &screen);
            }
        }

        gpu.renderer.queue.submit(Some(encoder.finish()));
        gpu.renderer.queue.present(frame);

        for id in &full_output.textures_delta.free {
            gpu.egui.renderer.free_texture(id);
        }

        if self.movement_mode {
            let delta = std::mem::take(&mut self.mouse_delta);
            gpu.renderer.on_update(delta, gpu.egui.winit.egui_ctx());
        }

        if gpu
            .egui
            .winit
            .egui_ctx()
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
        let mut descriptor =
            InstanceDescriptor::new_with_display_handle(Box::new(target.owned_display_handle()));
        descriptor.backends = wgpu::Backends::VULKAN;
        let instance = wgpu::Instance::new(descriptor.with_env());

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

        let scene = define_render_scene();
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

        dbg!(triangle_count);
        dbg!(sub_object_count);
        dbg!(scene.objects.len());
        dbg!(scene.spheres.len());
        dbg!(scene.materials.len());

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

        // ################################ GPU COMPUTE PIPELINE #########################################

        let compute_module = unsafe {
            renderer.device.create_shader_module_trusted(
                wgpu::ShaderModuleDescriptor {
                    label: Some("compute_shader.wgsl"),
                    source: wgpu::ShaderSource::Wgsl(include_str!("compute_shader.wgsl").into()),
                },
                ShaderRuntimeChecks::unchecked(),
            )
        };

        let compute_pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Compute Pipeline Layout"),
                    bind_group_layouts: &[Some(&compute_bindgroup_layout)],
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
                    bind_group_layouts: &[Some(&bind_group_layout)],
                    immediate_size: 0,
                });

        let render_pipeline =
            create_render_pipeline(&renderer.device, &pipeline_layout, SURFACE_FORMAT);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: SURFACE_FORMAT,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::FifoRelaxed,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            color_space: wgpu::SurfaceColorSpace::Auto,
            view_formats: vec![SURFACE_FORMAT],
        };

        surface.configure(&renderer.device, &surface_config);

        let egui = EguiState {
            winit: egui_winit::State::new(
                egui::Context::default(),
                egui::ViewportId::ROOT,
                &window,
                None,
                None,
                None,
            ),
            renderer: egui_wgpu::Renderer::new(
                &renderer.device,
                surface_config.format,
                egui_wgpu::RendererOptions::default(),
            ),
        };

        Self {
            window,
            surface,
            renderer,
            compute_pipeline,
            compute_bind_group,
            sampler,
            bind_group,
            render_pipeline,
            surface_config,
            egui,
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;

        self.compute_bind_group = self.renderer.on_resize(&size);

        let (_, bind_group) = create_device_bindgroup(
            &self.renderer.device,
            &self.renderer.output_texture_view(),
            &self.sampler,
        );

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
) -> wgpu::RenderPass<'static> {
    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
    rpass.set_pipeline(render_pipeline);
    rpass.set_bind_group(0, bind_group, &[]);
    rpass.draw(0..6, 0..1);
    rpass.forget_lifetime()
}

fn create_render_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &PipelineLayout,
    swapchain_format: TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(include_wgsl!("render_shader.wgsl"));

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
    })
}

async fn create_adapter(instance: &wgpu::Instance, surface: &Surface<'_>) -> wgpu::Adapter {
    instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            // Request an adapter which can render to our surface
            compatible_surface: Some(surface),
            apply_limit_buckets: false,
        })
        .await
        .expect("Failed to find an appropriate adapter")
}

async fn generate_device_and_queue(adapter: &Adapter) -> (Device, Queue) {
    let adapter_limits = wgpu::Limits {
        max_storage_buffers_per_shader_stage: 8,
        ..wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits())
    };

    adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: adapter_limits,
            memory_hints: wgpu::MemoryHints::Performance,
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

const UI_MAX_WIDTH: f32 = 160.0;

// simple macro for making the UI more compact
macro_rules! create_drag_value {
    ($ui:expr, $value:expr, $speed:expr, $range:expr, $prefix:expr) => {{
        let dv = DragValue::new($value)
            .speed($speed)
            .range($range)
            .prefix($prefix);
        let saved = $ui.style().visuals.clone();
        let v = $ui.visuals_mut();
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(60, 63, 78));
        let width = $ui.available_width().min(UI_MAX_WIDTH);
        let changed = $ui.add_sized([width, 18.0], dv).changed();
        $ui.visuals_mut().widgets = saved.widgets;
        changed
    }};
}

fn action_button(ui: &mut egui::Ui, label: &str) -> bool {
    let saved = ui.style().visuals.widgets.clone();
    let v = ui.visuals_mut();
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(70, 92, 145);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(95, 122, 185);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(52, 68, 110);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(235, 238, 250));
    let width = UI_MAX_WIDTH / 2.0;
    let clicked = ui
        .add(
            egui::Button::new(egui::RichText::new(label).strong())
                .min_size([width, 22.0].into())
                .corner_radius(6),
        )
        .clicked();
    ui.visuals_mut().widgets = saved;
    ui.add_space(2.0);
    clicked
}

fn create_ui(ui: &mut egui::Ui, renderer: &mut Renderer, compute_per_second: &u32) {
    ui.visuals_mut().override_text_color = Some(Color32::from_rgb(200, 200, 200));

    let transparent_frame = Frame::new().fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200));

    let mut interacted = false;

    egui::Panel::right("side_panel")
        .resizable(false)
        .frame(transparent_frame)
        .show(ui, |ui| {
            ui.set_max_width(UI_MAX_WIDTH);

            ui.label(format!("samples/s: {}", compute_per_second));

            ui.vertical_centered(|ui| {
                let sky_color = &mut renderer.scene.environment_map.color;
                if let Some(sky_color) = sky_color {
                    ui.label("sky color:");
                    interacted |= ui
                        .color_edit_button_rgb(sky_color)
                        .on_hover_text("color")
                        .changed();
                }

                interacted |= ui
                    .checkbox(&mut renderer.accumulate, "light accumulation")
                    .changed();

                ui.add_space(10.0);

                ui.label("selected object:");
                ui.add(
                    egui::Slider::new(
                        &mut renderer.object_index,
                        0..=(renderer.scene.objects.len() - 1),
                    )
                    .integer(),
                );

                let current_object = &mut renderer.scene.objects[renderer.object_index];
                let coordinates = &mut current_object.transformation;

                ui.label("location:");
                ui.vertical(|ui| {
                    interacted |=
                        create_drag_value!(ui, &mut coordinates[0], 0.1, -400.0..=400.0, "X: ");
                    interacted |=
                        create_drag_value!(ui, &mut coordinates[1], 0.1, -400.0..=10.0, "Y: ");
                    interacted |=
                        create_drag_value!(ui, &mut coordinates[2], 0.1, -400.0..=400.0, "Z: ");
                });

                ui.add_space(10.0);

                let rotation = &mut current_object.rotation;

                ui.label("rotation:");
                ui.vertical(|ui| {
                    interacted |=
                        create_drag_value!(ui, &mut rotation[0], 1.0, -180.0..=180.0, "X: ");
                    interacted |=
                        create_drag_value!(ui, &mut rotation[1], 1.0, -180.0..=180.0, "Y: ");
                    interacted |=
                        create_drag_value!(ui, &mut rotation[2], 1.0, -180.0..=180.0, "Z: ");
                });

                // sliders for scale
                let object_size = &mut current_object.scale;
                interacted |= create_drag_value!(ui, object_size, 0.01, 0.1..=100.0, "scale: ");

                if action_button(ui, "return to surface") {
                    current_object.set_model_to_surface();
                    interacted = true;
                }

                if action_button(ui, "reset rotation") {
                    current_object.reset_rotation();
                    interacted = true;
                }

                let material_index = current_object.material_index as usize;
                interacted |= ui_material_selection(renderer, material_index, ui);

                ui.add_space(30.0);

                ui.label("selected sphere:");
                ui.add(
                    egui::Slider::new(
                        &mut renderer.sphere_index,
                        0..=(renderer.scene.spheres.len() - 1),
                    )
                    .integer(),
                );

                let index = renderer.sphere_index;
                let current_sphere = &mut renderer.scene.spheres[index];

                // X Y Z sliders
                let sphere_position = &mut current_sphere.position;
                ui.vertical(|ui| {
                    interacted |=
                        create_drag_value!(ui, &mut sphere_position[0], 0.1, -400.0..=400.0, "X: ");
                    interacted |=
                        create_drag_value!(ui, &mut sphere_position[1], 0.1, -400.0..=10.0, "Y: ");
                    interacted |=
                        create_drag_value!(ui, &mut sphere_position[2], 0.1, -400.0..=400.0, "Z: ");
                });

                // sliders for radius
                let sphere_radius = &mut current_sphere.radius;
                interacted |= create_drag_value!(ui, sphere_radius, 0.01, 0.1..=50.0, "radius: ");

                let material_index = current_sphere.material_index as usize;
                interacted |= ui_material_selection(renderer, material_index, ui);
            });
        });

    if interacted {
        renderer.update_scene()
    }
}

fn ui_material_selection(
    renderer: &mut Renderer,
    material_index: usize,
    ui: &mut egui::Ui,
) -> bool {
    let mut interacted = false;

    ui.label("object material: ");
    let current_material = &mut renderer.scene.materials[material_index];

    let texture_index = current_material.texture_index;
    let current_image = &mut renderer.scene.image_textures[texture_index as usize];

    if let Some(color) = &mut current_image.color {
        interacted |= ui
            .color_edit_button_rgb(color)
            .on_hover_text("color")
            .changed();
    }

    interacted |= create_drag_value!(
        ui,
        &mut current_material.emission_power,
        0.2,
        0.0..=200.0,
        "emission power: "
    );

    interacted |= create_drag_value!(
        ui,
        &mut current_material.roughness,
        0.01,
        0.0..=1.0,
        "roughness: "
    );

    interacted |= create_drag_value!(
        ui,
        &mut current_material.specular,
        0.01,
        0.0..=1.0,
        "specular: "
    );

    interacted |= create_drag_value!(
        ui,
        &mut current_material.specular_scatter,
        0.01,
        0.0..=0.5,
        "specular scatter: "
    );

    interacted |= create_drag_value!(ui, &mut current_material.glass, 0.01, 0.0..=1.0, "glass: ");

    interacted |= create_drag_value!(
        ui,
        &mut current_material.refraction_index,
        0.01,
        0.0..=5.0,
        "refraction index: "
    );

    interacted
}
