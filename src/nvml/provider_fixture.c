// Small stand-in for the Linux NVML ABI. Each test loads its own copy.
#include <stdint.h>
#include <stdio.h>
#include <string.h>

static const char *scenario = "normal";
static int memory_result;
static unsigned init_count, shutdown_count, legacy_count;
static int initialized;

void fixture_configure(const char *name, int result) {
    scenario = name;
    memory_result = result;
}

void fixture_counts(unsigned *init, unsigned *shutdown, unsigned *legacy) {
    *init = init_count;
    *shutdown = shutdown_count;
    *legacy = legacy_count;
}

int nvmlInit_v2(void) {
    ++init_count;
    if (strcmp(scenario, "driver absent") == 0) return 9;
    if (initialized) return 5;
    initialized = 1;
    return 0;
}

int nvmlShutdown(void) {
    ++shutdown_count;
    initialized = 0;
    return 0;
}

int nvmlDeviceGetCount_v2(unsigned *count) {
    if (!initialized) return 1;
    if (strcmp(scenario, "enumeration failed") == 0) return 999;
    *count = strcmp(scenario, "zero") == 0 ? 0 : 2;
    return 0;
}

int nvmlDeviceGetHandleByIndex_v2(unsigned index, void **handle) {
    if (strcmp(scenario, "handle lost") == 0 && index == 0) return 15;
    uintptr_t id = strcmp(scenario, "reorder") == 0 ? 2 - index : 1 + index;
    *handle = (void *)id;
    return 0;
}

int nvmlDeviceGetUUID(void *device, char *output, unsigned size) {
    if (strcmp(scenario, "no UUID") == 0) return 3;
    snprintf(output, size, "GPU-00000000-0000-0000-0000-%012u",
             (unsigned)(uintptr_t)device);
    return 0;
}

int nvmlDeviceGetName(void *device, char *output, unsigned size) {
    (void)device;
    snprintf(output, size, "Identical model name");
    return 0;
}

int nvmlDeviceGetTemperature(void *device, unsigned sensor, unsigned *value) {
    if (sensor != 0) return 2;
    *value = 60 + (unsigned)(uintptr_t)device;
    return strcmp(scenario, "query lost") == 0 && (uintptr_t)device == 1 ? 15 : 0;
}

struct utilization { unsigned gpu, memory; };
int nvmlDeviceGetUtilizationRates(void *device, struct utilization *value) {
    (void)device;
    value->gpu = 45;
    value->memory = 11;
    return 0;
}

struct memory_v2 { unsigned version; uint64_t total, reserved, free, used; };
int nvmlDeviceGetMemoryInfo_v2(void *device, struct memory_v2 *value) {
    (void)device;
    if (value->version != (sizeof(*value) | (2U << 24))) return 25;
    value->total = UINT64_C(8589934592);
    value->reserved = 1024;
    value->free = value->total - 3072;
    value->used = strcmp(scenario, "large unsigned") == 0 ? UINT64_MAX : 2048;
    return memory_result;
}

struct memory { uint64_t total, free, used; };
int nvmlDeviceGetMemoryInfo(void *device, struct memory *value) {
    (void)device;
    ++legacy_count;
    value->total = UINT64_C(8589934592);
    value->free = value->total - 3072;
    value->used = 3072;
    return 0;
}

int nvmlDeviceGetClockInfo(void *device, unsigned selector, unsigned *value) {
    (void)device;
    if (selector != 0 && selector != 2) return 2;
    *value = selector == 0 ? 734 : 2504;
    return 0;
}

int nvmlDeviceGetPowerUsage(void *device, unsigned *value) {
    (void)device;
    *value = 25750;
    return 0;
}

// Deliberately omit nvmlDeviceGetFanSpeed to exercise symbol-level fallback.
int nvmlDeviceGetPerformanceState(void *device, unsigned *value) {
    *value = (uintptr_t)device == 1 ? 3 : 32;
    return 0;
}
