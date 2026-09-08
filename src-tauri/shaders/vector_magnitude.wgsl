// 矢量模量算子（T20）：对 vec4 输入取 xyz 分量的欧氏长度。
// 每线程处理一个矢量；越界线程直接返回。
struct Vectors {
    data: array<vec4<f32>>,
}
@group(0) @binding(0) var<storage, read> input: Vectors;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let index = gid.x;
    if (index >= arrayLength(&input.data)) {
        return;
    }
    output[index] = length(input.data[index].xyz);
}
