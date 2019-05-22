use gl;
use crate::program;
use crate::buffer::*;

// TODO: Make non-static object
pub fn bind(gl: &gl::Gl, program: &program::Program)
{
    let buffer =
        unsafe {
            static mut FULL_SCREEN__QUAD: Option<StaticVertexBuffer> = None;
            if FULL_SCREEN__QUAD.is_none()
            {
                let positions = vec![
                    -3.0, -1.0, 0.0,
                    3.0, -1.0, 0.0,
                    0.0, 2.0, 0.0
                ];
                let uvs = vec![
                    -1.0, 0.0,
                    2.0, 0.0,
                    0.5, 1.5
                ];
                FULL_SCREEN__QUAD = Some(StaticVertexBuffer::new_with_vec3_vec2(&gl, &positions, &uvs).unwrap());
            }
            FULL_SCREEN__QUAD.as_ref().unwrap()
        };

    buffer.bind();
    program.use_attribute_vec3_float(buffer, "position", 0).unwrap();
    program.use_attribute_vec2_float(buffer, "uv_coordinate", 1).unwrap();
}

pub fn render(program: &program::Program)
{
    program.draw_arrays(3);
}