use crate::context::{consts, Context};
use crate::core::*;
use crate::definition::*;
use crate::math::*;

pub use crate::{Format, Interpolation, Wrapping};

///
/// A 2D texture, basically an image that is transferred to the GPU.
/// For a texture that can be rendered into, see [ColorTargetTexture2D](crate::ColorTargetTexture2D).
///
pub struct Texture2D {
    context: Context,
    id: crate::context::Texture,
    width: u32,
    height: u32,
    format: Format,
    number_of_mip_maps: u32,
}

impl Texture2D {
    ///
    /// Construcs a new texture with the given data.
    ///
    pub fn new<T: TextureDataType>(
        context: &Context,
        cpu_texture: &CPUTexture<T>,
    ) -> Result<Texture2D, Error> {
        let id = generate(context)?;
        let number_of_mip_maps = calculate_number_of_mip_maps(
            cpu_texture.mip_map_filter,
            cpu_texture.width,
            cpu_texture.height,
            1,
        );
        set_parameters(
            context,
            &id,
            consts::TEXTURE_2D,
            cpu_texture.min_filter,
            cpu_texture.mag_filter,
            if number_of_mip_maps == 1 {
                None
            } else {
                cpu_texture.mip_map_filter
            },
            cpu_texture.wrap_s,
            cpu_texture.wrap_t,
            None,
        );
        context.tex_storage_2d(
            consts::TEXTURE_2D,
            number_of_mip_maps,
            T::internal_format(cpu_texture.format)?,
            cpu_texture.width as u32,
            cpu_texture.height as u32,
        );
        let mut tex = Self {
            context: context.clone(),
            id,
            width: cpu_texture.width,
            height: cpu_texture.height,
            format: cpu_texture.format,
            number_of_mip_maps,
        };
        tex.fill(&cpu_texture.data)?;
        Ok(tex)
    }

    ///
    /// Fills this texture with the given data.
    ///
    /// # Errors
    /// Return an error if the length of the data array is smaller or bigger than the necessary number of bytes to fill the entire texture.
    ///
    pub fn fill<T: TextureDataType>(&mut self, data: &[T]) -> Result<(), Error> {
        check_data_length(self.width, self.height, 1, self.format, data.len())?;
        self.context.bind_texture(consts::TEXTURE_2D, &self.id);
        T::fill(
            &self.context,
            consts::TEXTURE_2D,
            self.width(),
            self.height(),
            self.format,
            data,
        );
        self.generate_mip_maps();
        Ok(())
    }

    pub(crate) fn generate_mip_maps(&self) {
        if self.number_of_mip_maps > 1 {
            self.context.bind_texture(consts::TEXTURE_2D, &self.id);
            self.context.generate_mipmap(consts::TEXTURE_2D);
        }
    }
}

impl Texture for Texture2D {
    fn bind(&self, location: u32) {
        bind_at(&self.context, &self.id, consts::TEXTURE_2D, location);
    }
    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
}

impl Drop for Texture2D {
    fn drop(&mut self) {
        self.context.delete_texture(&self.id);
    }
}

///
/// A 2D color texture that can be rendered into and read from.
///
/// **Note:** [Depth test](crate::DepthTestType) is disabled if not also writing to a depth texture.
/// Use a [RenderTarget](crate::RenderTarget) to write to both color and depth.
///
pub struct ColorTargetTexture2D<T: TextureDataType> {
    context: Context,
    id: crate::context::Texture,
    width: u32,
    height: u32,
    number_of_mip_maps: u32,
    format: Format,
    _dummy: T,
}

impl<T: TextureDataType> ColorTargetTexture2D<T> {
    ///
    /// Constructs a new 2D color target texture.
    ///
    pub fn new(
        context: &Context,
        width: u32,
        height: u32,
        min_filter: Interpolation,
        mag_filter: Interpolation,
        mip_map_filter: Option<Interpolation>,
        wrap_s: Wrapping,
        wrap_t: Wrapping,
        format: Format,
    ) -> Result<Self, Error> {
        let id = generate(context)?;
        let number_of_mip_maps = calculate_number_of_mip_maps(mip_map_filter, width, height, 1);
        set_parameters(
            context,
            &id,
            consts::TEXTURE_2D,
            min_filter,
            mag_filter,
            if number_of_mip_maps == 1 {
                None
            } else {
                mip_map_filter
            },
            wrap_s,
            wrap_t,
            None,
        );
        context.tex_storage_2d(
            consts::TEXTURE_2D,
            number_of_mip_maps,
            T::internal_format(format)?,
            width,
            height,
        );
        Ok(Self {
            context: context.clone(),
            id,
            width,
            height,
            number_of_mip_maps,
            format,
            _dummy: T::default(),
        })
    }

    ///
    /// Renders whatever rendered in the `render` closure into the texture.
    /// Before writing, the texture is cleared based on the given clear state.
    ///
    /// **Note:** [Depth test](crate::DepthTestType) is disabled if not also writing to a depth texture.
    /// Use a [RenderTarget](crate::RenderTarget) to write to both color and depth.
    ///
    pub fn write<F: FnOnce() -> Result<(), Error>>(
        &self,
        clear_state: ClearState,
        render: F,
    ) -> Result<(), Error> {
        RenderTarget::<T>::new_color(&self.context, &self)?.write(clear_state, render)
    }

    ///
    /// Copies the content of the color texture to the specified [destination](crate::CopyDestination) at the given viewport.
    /// Will only copy the channels specified by the write mask.
    ///
    /// # Errors
    /// Will return an error if the destination is a depth texture.
    ///
    pub fn copy_to(
        &self,
        destination: CopyDestination<T>,
        viewport: Viewport,
        write_mask: WriteMask,
    ) -> Result<(), Error> {
        RenderTarget::new_color(&self.context, &self)?.copy_to(destination, viewport, write_mask)
    }

    ///
    /// Returns the color values of the pixels in this color texture inside the given viewport.
    ///
    /// **Note:** Only works for the RGBA format.
    ///
    /// # Errors
    /// Will return an error if the color texture is not RGBA format.
    ///
    pub fn read(&self, viewport: Viewport) -> Result<Vec<T>, Error> {
        if self.format != Format::RGBA {
            Err(Error::TextureError {
                message: "Cannot read color from anything else but an RGBA texture.".to_owned(),
            })?;
        }

        let mut pixels = vec![
            T::default();
            viewport.width as usize
                * viewport.height as usize
                * self.format.color_channel_count() as usize
        ];
        let render_target = RenderTarget::new_color(&self.context, &self)?;
        render_target.bind(consts::DRAW_FRAMEBUFFER)?;
        render_target.bind(consts::READ_FRAMEBUFFER)?;
        T::read(&self.context, viewport, self.format, &mut pixels);
        Ok(pixels)
    }

    pub(super) fn generate_mip_maps(&self) {
        if self.number_of_mip_maps > 1 {
            self.context.bind_texture(consts::TEXTURE_2D, &self.id);
            self.context.generate_mipmap(consts::TEXTURE_2D);
        }
    }

    pub(super) fn bind_as_color_target(&self, channel: u32) {
        self.context.framebuffer_texture_2d(
            consts::FRAMEBUFFER,
            consts::COLOR_ATTACHMENT0 + channel,
            consts::TEXTURE_2D,
            &self.id,
            0,
        );
    }
}

impl<T: TextureDataType> Texture for ColorTargetTexture2D<T> {
    fn bind(&self, location: u32) {
        bind_at(&self.context, &self.id, consts::TEXTURE_2D, location);
    }
    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
}

impl<T: TextureDataType> Drop for ColorTargetTexture2D<T> {
    fn drop(&mut self) {
        self.context.delete_texture(&self.id);
    }
}

///
/// Type of formats for depth render targets ([DepthTargetTexture2D](crate::DepthTargetTexture2D) and
/// [DepthTargetTexture2DArray](crate::DepthTargetTexture2DArray)).
///
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum DepthFormat {
    Depth16,
    Depth24,
    Depth32F,
}

///
/// A 2D depth texture that can be rendered into and read from. See also [RenderTarget](crate::RenderTarget).
///
pub struct DepthTargetTexture2D {
    context: Context,
    id: crate::context::Texture,
    width: u32,
    height: u32,
}

impl DepthTargetTexture2D {
    ///
    /// Constructs a new 2D depth target texture.
    ///
    pub fn new(
        context: &Context,
        width: u32,
        height: u32,
        wrap_s: Wrapping,
        wrap_t: Wrapping,
        format: DepthFormat,
    ) -> Result<Self, Error> {
        let id = generate(context)?;
        set_parameters(
            context,
            &id,
            consts::TEXTURE_2D,
            Interpolation::Nearest,
            Interpolation::Nearest,
            None,
            wrap_s,
            wrap_t,
            None,
        );
        context.tex_storage_2d(
            consts::TEXTURE_2D,
            1,
            internal_format_from_depth(format),
            width as u32,
            height as u32,
        );
        Ok(Self {
            context: context.clone(),
            id,
            width,
            height,
        })
    }

    ///
    /// Write the depth of whatever rendered in the `render` closure into the texture.
    /// Before writing, the texture is cleared based on the given clear state.
    ///
    pub fn write<F: FnOnce() -> Result<(), Error>>(
        &self,
        clear_state: Option<f32>,
        render: F,
    ) -> Result<(), Error> {
        RenderTarget::<f32>::new_depth(&self.context, &self)?.write(
            ClearState {
                depth: clear_state,
                ..ClearState::none()
            },
            render,
        )
    }

    ///
    /// Copies the content of the depth texture to the specified [destination](crate::CopyDestination) at the given viewport.
    ///
    /// # Errors
    /// Will return an error if the destination is a color texture.
    ///
    pub fn copy_to<T: TextureDataType>(
        &self,
        destination: CopyDestination<T>,
        viewport: Viewport,
    ) -> Result<(), Error> {
        RenderTarget::new_depth(&self.context, &self)?.copy_to(
            destination,
            viewport,
            WriteMask::DEPTH,
        )
    }

    pub(super) fn bind_as_depth_target(&self) {
        self.context.framebuffer_texture_2d(
            consts::FRAMEBUFFER,
            consts::DEPTH_ATTACHMENT,
            consts::TEXTURE_2D,
            &self.id,
            0,
        );
    }
}

impl Texture for DepthTargetTexture2D {
    fn bind(&self, location: u32) {
        bind_at(&self.context, &self.id, consts::TEXTURE_2D, location);
    }
    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
}

impl Drop for DepthTargetTexture2D {
    fn drop(&mut self) {
        self.context.delete_texture(&self.id);
    }
}

///
/// A texture that covers all 6 sides of a cube.
///
pub struct TextureCubeMap<T: TextureDataType> {
    context: Context,
    id: crate::context::Texture,
    width: u32,
    height: u32,
    format: Format,
    number_of_mip_maps: u32,
    _dummy: T,
}

impl<T: TextureDataType> TextureCubeMap<T> {
    pub fn new(context: &Context, cpu_texture: &CPUTexture<T>) -> Result<TextureCubeMap<T>, Error> {
        let id = generate(context)?;
        let number_of_mip_maps = calculate_number_of_mip_maps(
            cpu_texture.mip_map_filter,
            cpu_texture.width,
            cpu_texture.height,
            1,
        );
        set_parameters(
            context,
            &id,
            consts::TEXTURE_CUBE_MAP,
            cpu_texture.min_filter,
            cpu_texture.mag_filter,
            if number_of_mip_maps == 1 {
                None
            } else {
                cpu_texture.mip_map_filter
            },
            cpu_texture.wrap_s,
            cpu_texture.wrap_t,
            Some(cpu_texture.wrap_r),
        );
        context.bind_texture(consts::TEXTURE_CUBE_MAP, &id);
        context.tex_storage_2d(
            consts::TEXTURE_CUBE_MAP,
            number_of_mip_maps,
            T::internal_format(cpu_texture.format)?,
            cpu_texture.width,
            cpu_texture.height,
        );
        let mut texture = Self {
            context: context.clone(),
            id,
            width: cpu_texture.width,
            height: cpu_texture.height,
            format: cpu_texture.format,
            number_of_mip_maps,
            _dummy: T::default(),
        };
        texture.fill(&cpu_texture.data)?;
        Ok(texture)
    }

    // data contains 6 images in the following order; right, left, top, bottom, front, back
    pub fn fill(&mut self, data: &[T]) -> Result<(), Error> {
        let offset = data.len() / 6;
        check_data_length(self.width, self.height, 1, self.format, offset)?;
        self.context
            .bind_texture(consts::TEXTURE_CUBE_MAP, &self.id);
        for i in 0..6 {
            T::fill(
                &self.context,
                consts::TEXTURE_CUBE_MAP_POSITIVE_X + i as u32,
                self.width,
                self.height,
                self.format,
                &data[i * offset..(i + 1) * offset],
            );
        }
        self.generate_mip_maps();
        Ok(())
    }

    pub(crate) fn generate_mip_maps(&self) {
        if self.number_of_mip_maps > 1 {
            self.context
                .bind_texture(consts::TEXTURE_CUBE_MAP, &self.id);
            self.context.generate_mipmap(consts::TEXTURE_CUBE_MAP);
        }
    }
}

impl<T: TextureDataType> TextureCube for TextureCubeMap<T> {
    fn bind(&self, location: u32) {
        bind_at(&self.context, &self.id, consts::TEXTURE_CUBE_MAP, location);
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }
}

impl<T: TextureDataType> Drop for TextureCubeMap<T> {
    fn drop(&mut self) {
        self.context.delete_texture(&self.id);
    }
}

///
/// A array of 2D color textures that can be rendered into.
///
/// **Note:** [Depth test](crate::DepthTestType) is disabled if not also writing to a depth texture array.
/// Use a [RenderTargetArray](crate::RenderTargetArray) to write to both color and depth.
///
pub struct ColorTargetTexture2DArray<T: TextureDataType> {
    context: Context,
    id: crate::context::Texture,
    width: u32,
    height: u32,
    depth: u32,
    number_of_mip_maps: u32,
    _dummy: T,
}

impl<T: TextureDataType> ColorTargetTexture2DArray<T> {
    pub fn new(
        context: &Context,
        width: u32,
        height: u32,
        depth: u32,
        min_filter: Interpolation,
        mag_filter: Interpolation,
        mip_map_filter: Option<Interpolation>,
        wrap_s: Wrapping,
        wrap_t: Wrapping,
        format: Format,
    ) -> Result<Self, Error> {
        let id = generate(context)?;
        let number_of_mip_maps = calculate_number_of_mip_maps(mip_map_filter, width, height, depth);
        set_parameters(
            context,
            &id,
            consts::TEXTURE_2D_ARRAY,
            min_filter,
            mag_filter,
            if number_of_mip_maps == 1 {
                None
            } else {
                mip_map_filter
            },
            wrap_s,
            wrap_t,
            None,
        );
        context.bind_texture(consts::TEXTURE_2D_ARRAY, &id);
        context.tex_storage_3d(
            consts::TEXTURE_2D_ARRAY,
            number_of_mip_maps,
            T::internal_format(format)?,
            width,
            height,
            depth,
        );
        Ok(Self {
            context: context.clone(),
            id,
            width,
            height,
            depth,
            number_of_mip_maps,
            _dummy: T::default(),
        })
    }

    ///
    /// Renders whatever rendered in the `render` closure into the textures defined by the input parameters `color_layers`.
    /// Output at location *i* defined in the fragment shader is written to the color texture layer at the *ith* index in `color_layers`.
    /// Before writing, the textures are cleared based on the given clear state.
    ///
    /// **Note:** [Depth test](crate::DepthTestType) is disabled if not also writing to a depth texture array.
    /// Use a [RenderTargetArray](crate::RenderTargetArray) to write to both color and depth.
    ///
    pub fn write<F: FnOnce() -> Result<(), Error>>(
        &self,
        color_layers: &[u32],
        clear_state: ClearState,
        render: F,
    ) -> Result<(), Error> {
        RenderTargetArray::new_color(&self.context, &self)?.write(
            color_layers,
            0,
            clear_state,
            render,
        )
    }

    ///
    /// Copies the content of the color texture at the given layer to the specified [destination](crate::CopyDestination) at the given viewport.
    /// Will only copy the channels specified by the write mask.
    ///
    /// # Errors
    /// Will return an error if the destination is a depth texture.
    ///
    pub fn copy_to(
        &self,
        color_layer: u32,
        destination: CopyDestination<T>,
        viewport: Viewport,
        write_mask: WriteMask,
    ) -> Result<(), Error> {
        RenderTargetArray::<T>::new_color(&self.context, &self)?.copy_to(
            color_layer,
            0,
            destination,
            viewport,
            write_mask,
        )
    }

    pub(crate) fn generate_mip_maps(&self) {
        if self.number_of_mip_maps > 1 {
            self.context
                .bind_texture(consts::TEXTURE_2D_ARRAY, &self.id);
            self.context.generate_mipmap(consts::TEXTURE_2D_ARRAY);
        }
    }

    pub(crate) fn bind_as_color_target(&self, layer: u32, channel: u32) {
        self.context.framebuffer_texture_layer(
            consts::DRAW_FRAMEBUFFER,
            consts::COLOR_ATTACHMENT0 + channel,
            &self.id,
            0,
            layer,
        );
    }
}

impl<T: TextureDataType> TextureArray for ColorTargetTexture2DArray<T> {
    fn bind(&self, location: u32) {
        bind_at(&self.context, &self.id, consts::TEXTURE_2D_ARRAY, location);
    }
    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
    fn depth(&self) -> u32 {
        self.depth
    }
}

impl<T: TextureDataType> Drop for ColorTargetTexture2DArray<T> {
    fn drop(&mut self) {
        self.context.delete_texture(&self.id);
    }
}

///
/// An array of 2D depth textures that can be rendered into and read from. See also [RenderTargetArray](crate::RenderTargetArray).
///
pub struct DepthTargetTexture2DArray {
    context: Context,
    id: crate::context::Texture,
    width: u32,
    height: u32,
    depth: u32,
}

impl DepthTargetTexture2DArray {
    pub fn new(
        context: &Context,
        width: u32,
        height: u32,
        depth: u32,
        wrap_s: Wrapping,
        wrap_t: Wrapping,
        format: DepthFormat,
    ) -> Result<Self, Error> {
        let id = generate(context)?;
        set_parameters(
            context,
            &id,
            consts::TEXTURE_2D_ARRAY,
            Interpolation::Nearest,
            Interpolation::Nearest,
            None,
            wrap_s,
            wrap_t,
            None,
        );
        context.bind_texture(consts::TEXTURE_2D_ARRAY, &id);
        context.tex_storage_3d(
            consts::TEXTURE_2D_ARRAY,
            1,
            internal_format_from_depth(format),
            width,
            height,
            depth,
        );
        Ok(Self {
            context: context.clone(),
            id,
            width,
            height,
            depth,
        })
    }

    ///
    /// Writes the depth of whatever rendered in the `render` closure into the depth texture defined by the input parameter `depth_layer`.
    /// Before writing, the texture is cleared based on the given clear state.
    ///
    pub fn write<F: FnOnce() -> Result<(), Error>>(
        &self,
        depth_layer: u32,
        clear_state: Option<f32>,
        render: F,
    ) -> Result<(), Error> {
        RenderTargetArray::<u8>::new_depth(&self.context, &self)?.write(
            &[],
            depth_layer,
            ClearState {
                depth: clear_state,
                ..ClearState::none()
            },
            render,
        )
    }

    ///
    /// Copies the content of the depth texture at the given layer to the specified [destination](crate::CopyDestination) at the given viewport.
    ///
    /// # Errors
    /// Will return an error if the destination is a color texture.
    ///
    pub fn copy_to<T: TextureDataType>(
        &self,
        depth_layer: u32,
        destination: CopyDestination<T>,
        viewport: Viewport,
    ) -> Result<(), Error> {
        RenderTargetArray::new_depth(&self.context, &self)?.copy_to(
            0,
            depth_layer,
            destination,
            viewport,
            WriteMask::DEPTH,
        )
    }

    pub(crate) fn bind_as_depth_target(&self, layer: u32) {
        self.context.framebuffer_texture_layer(
            consts::DRAW_FRAMEBUFFER,
            consts::DEPTH_ATTACHMENT,
            &self.id,
            0,
            layer as u32,
        );
    }
}

impl TextureArray for DepthTargetTexture2DArray {
    fn bind(&self, location: u32) {
        bind_at(&self.context, &self.id, consts::TEXTURE_2D_ARRAY, location);
    }
    fn width(&self) -> u32 {
        self.width
    }
    fn height(&self) -> u32 {
        self.height
    }
    fn depth(&self) -> u32 {
        self.depth
    }
}

impl Drop for DepthTargetTexture2DArray {
    fn drop(&mut self) {
        self.context.delete_texture(&self.id);
    }
}
