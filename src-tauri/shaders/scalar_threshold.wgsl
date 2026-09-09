// 派生算子：阈值掩码 v >= threshold ? 1 : 0
struct Params {
    threshold: f32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> params: Params;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= arrayLength(&input)) {
        return;
    }
    output[i] = select(0.0, 1.0, input[i] >= params.threshold);
}
