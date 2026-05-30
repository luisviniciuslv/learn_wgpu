use winit::window::Window; fn test(w: &Window) { w.set_aspect_ratio(Some(16.0 / 9.0)); w.set_min_inner_size(Some(winit::dpi::LogicalSize::new(1.0, 1.0))); }
