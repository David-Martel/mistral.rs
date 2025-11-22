// Simple CUDA test file
__global__ void test_kernel(float* data) {
    int idx = threadIdx.x + blockIdx.x * blockDim.x;
    data[idx] = data[idx] * 2.0f;
}
