#include <hip/hip_runtime_api.h>

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <fstream>
#include <iostream>
#include <string>
#include <vector>

namespace {

constexpr std::size_t kElements = 3072;
constexpr unsigned int kBlockSize = 256;
constexpr unsigned int kBlockCount = 2;

bool hip_ok(hipError_t status, const char *operation) {
  if (status == hipSuccess) {
    return true;
  }
  std::cerr << operation << ": " << hipGetErrorString(status) << '\n';
  return false;
}

std::uint16_t float_to_bf16_rne(float value) {
  std::uint32_t bits = 0;
  static_assert(sizeof(bits) == sizeof(value));
  std::memcpy(&bits, &value, sizeof(bits));
  const std::uint32_t rounding = 0x7fffU + ((bits >> 16U) & 1U);
  return static_cast<std::uint16_t>((bits + rounding) >> 16U);
}

float bf16_to_float(std::uint16_t value) {
  const std::uint32_t bits = static_cast<std::uint32_t>(value) << 16U;
  float result = 0.0F;
  std::memcpy(&result, &bits, sizeof(result));
  return result;
}

std::uint16_t ordered_bf16(std::uint16_t value) {
  return (value & 0x8000U) != 0U ? static_cast<std::uint16_t>(~value)
                                 : static_cast<std::uint16_t>(value ^ 0x8000U);
}

unsigned int bf16_ulp_distance(std::uint16_t left, std::uint16_t right) {
  const unsigned int ordered_left = ordered_bf16(left);
  const unsigned int ordered_right = ordered_bf16(right);
  return ordered_left > ordered_right ? ordered_left - ordered_right
                                      : ordered_right - ordered_left;
}

float stable_swiglu(float gate, float up) {
  const bool nonnegative = gate >= 0.0F;
  const float exponent = std::exp(nonnegative ? -gate : gate);
  const float numerator = nonnegative ? 1.0F : exponent;
  return (gate * (numerator / (1.0F + exponent))) * up;
}

bool read_file(const char *path, std::vector<char> *bytes) {
  std::ifstream input(path, std::ios::binary | std::ios::ate);
  if (!input) {
    std::cerr << "cannot open HSACO: " << path << '\n';
    return false;
  }
  const std::streamsize length = input.tellg();
  if (length <= 0 || length > 4 * 1024 * 1024) {
    std::cerr << "HSACO length is outside the qualification bound\n";
    return false;
  }
  bytes->resize(static_cast<std::size_t>(length));
  input.seekg(0, std::ios::beg);
  return static_cast<bool>(input.read(bytes->data(), length));
}

} // namespace

int main(int argc, char **argv) {
  if (argc != 2) {
    std::cerr << "usage: ferric-swiglu-hip-numeric <hsaco>\n";
    return 2;
  }

  hipDeviceProp_t properties{};
  if (!hip_ok(hipSetDevice(0), "hipSetDevice") ||
      !hip_ok(hipGetDeviceProperties(&properties, 0),
              "hipGetDeviceProperties")) {
    return 1;
  }
  const std::string architecture(properties.gcnArchName);
  if (architecture.rfind("gfx942", 0) != 0) {
    std::cerr << "qualification requires gfx942, observed " << architecture
              << '\n';
    return 1;
  }

  std::vector<char> image;
  if (!read_file(argv[1], &image)) {
    return 1;
  }

  hipModule_t module = nullptr;
  hipFunction_t function = nullptr;
  if (!hip_ok(hipModuleLoadData(&module, image.data()), "hipModuleLoadData") ||
      !hip_ok(
          hipModuleGetFunction(&function, module, "qwen3_swiglu_bf16_f32_v1"),
          "hipModuleGetFunction")) {
    return 1;
  }

  std::vector<std::uint16_t> gate(kElements);
  std::vector<std::uint16_t> up(kElements);
  std::vector<std::uint16_t> output(kElements, 0x7fc1U);
  std::vector<std::uint16_t> expected(kElements);
  for (std::size_t index = 0; index < kElements; ++index) {
    float gate_value =
        static_cast<float>(static_cast<int>(index % 129U) - 64) / 8.0F;
    float up_value =
        static_cast<float>(static_cast<int>((index * 17U) % 65U) - 32) / 8.0F;
    if (index == 0) {
      gate_value = 0.0F;
      up_value = 1.0F;
    } else if (index == 1) {
      gate_value = -0.0F;
      up_value = 1.0F;
    } else if (index == 2) {
      gate_value = 1.0F;
      up_value = 2.0F;
    } else if (index == 3) {
      gate_value = -1.0F;
      up_value = 2.0F;
    }
    gate[index] = float_to_bf16_rne(gate_value);
    up[index] = float_to_bf16_rne(up_value);
    expected[index] = float_to_bf16_rne(
        stable_swiglu(bf16_to_float(gate[index]), bf16_to_float(up[index])));
  }

  std::uint16_t *device_gate = nullptr;
  std::uint16_t *device_up = nullptr;
  std::uint16_t *device_output = nullptr;
  const std::size_t byte_length = kElements * sizeof(std::uint16_t);
  if (!hip_ok(hipMalloc(reinterpret_cast<void **>(&device_gate), byte_length),
              "hipMalloc(gate)") ||
      !hip_ok(hipMalloc(reinterpret_cast<void **>(&device_up), byte_length),
              "hipMalloc(up)") ||
      !hip_ok(hipMalloc(reinterpret_cast<void **>(&device_output), byte_length),
              "hipMalloc(output)") ||
      !hip_ok(hipMemcpy(device_gate, gate.data(), byte_length,
                        hipMemcpyHostToDevice),
              "hipMemcpy(gate)") ||
      !hip_ok(
          hipMemcpy(device_up, up.data(), byte_length, hipMemcpyHostToDevice),
          "hipMemcpy(up)") ||
      !hip_ok(hipMemset(device_output, 0xa5, byte_length),
              "hipMemset(output)")) {
    return 1;
  }

  std::size_t gate_length = kElements;
  std::size_t up_length = kElements;
  std::size_t output_length = kElements;
  void *arguments[] = {&device_gate, &gate_length,   &device_up,
                       &up_length,   &device_output, &output_length};
  if (!hip_ok(hipModuleLaunchKernel(function, kBlockCount, 1, 1, kBlockSize, 1,
                                    1, 0, nullptr, arguments, nullptr),
              "hipModuleLaunchKernel") ||
      !hip_ok(hipDeviceSynchronize(), "hipDeviceSynchronize") ||
      !hip_ok(hipMemcpy(output.data(), device_output, byte_length,
                        hipMemcpyDeviceToHost),
              "hipMemcpy(output)")) {
    return 1;
  }

  unsigned int max_ulp = 0;
  std::size_t exact = 0;
  std::size_t mismatches = 0;
  for (std::size_t index = 0; index < kElements; ++index) {
    exact += output[index] == expected[index] ? 1U : 0U;
    const unsigned int ulp = bf16_ulp_distance(output[index], expected[index]);
    max_ulp = std::max(max_ulp, ulp);
    if (ulp > 1U) {
      if (mismatches < 8U) {
        std::cerr << "mismatch index=" << index
                  << " gate=" << bf16_to_float(gate[index])
                  << " up=" << bf16_to_float(up[index])
                  << " expected=" << bf16_to_float(expected[index])
                  << " actual=" << bf16_to_float(output[index])
                  << " ulp=" << ulp << '\n';
      }
      ++mismatches;
    }
  }

  bool cleanup_ok = hip_ok(hipFree(device_output), "hipFree(output)");
  cleanup_ok = hip_ok(hipFree(device_up), "hipFree(up)") && cleanup_ok;
  cleanup_ok = hip_ok(hipFree(device_gate), "hipFree(gate)") && cleanup_ok;
  cleanup_ok = hip_ok(hipModuleUnload(module), "hipModuleUnload") && cleanup_ok;

  std::cout << "architecture=" << architecture << " elements=" << kElements
            << " exact=" << exact << " max_ulp=" << max_ulp
            << " mismatches_gt_1ulp=" << mismatches << '\n';
  return mismatches == 0 && cleanup_ok ? 0 : 1;
}
