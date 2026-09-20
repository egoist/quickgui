package native

import (
	"fmt"
	"net/url"
	"os"
	"strings"
)

type AutoStartOptions struct {
	AppName          string   `json:"appName"`
	Executable       string   `json:"executable,omitempty"`
	Arguments        []string `json:"arguments,omitempty"`
	Mode             string   `json:"mode,omitempty"`
	BundleIdentifier string   `json:"bundleIdentifier,omitempty"`
}

type autoStartAPI struct{}

// AutoStart manages login startup through the native platform implementation.
var AutoStart autoStartAPI

func (autoStartAPI) IsSupported() (bool, error) {
	return callJSON[bool]("is-auto-start-supported", nil)
}

func (autoStartAPI) Enable(options AutoStartOptions, done func(error)) {
	invokeVoid("enable-auto-start", options, done)
}

func (autoStartAPI) Disable(options AutoStartOptions, done func(error)) {
	invokeVoid("disable-auto-start", options, done)
}

func (autoStartAPI) IsEnabled(options AutoStartOptions, done func(bool, error)) {
	invokeJSON("is-auto-start-enabled", options, done)
}

type ProtocolRegistrationOptions struct {
	Scheme     string   `json:"scheme"`
	AppName    string   `json:"appName"`
	AppID      string   `json:"appId"`
	Executable string   `json:"executable,omitempty"`
	Arguments  []string `json:"arguments,omitempty"`
}

type protocolAPI struct{}

var Protocol protocolAPI

func (protocolAPI) SupportsDynamicRegistration() (bool, error) {
	return callJSON[bool]("supports-dynamic-protocol-registration", nil)
}

func (protocolAPI) Register(options ProtocolRegistrationOptions, done func(bool, error)) {
	invokeJSON("register-protocol", options, done)
}

func (protocolAPI) Unregister(options ProtocolRegistrationOptions, done func(bool, error)) {
	invokeJSON("unregister-protocol", options, done)
}

func (protocolAPI) IsRegistered(options ProtocolRegistrationOptions, done func(bool, error)) {
	invokeJSON("is-protocol-registered", options, done)
}

type deepLinkAPI struct{ protocolAPI }

var DeepLink deepLinkAPI

func (deepLinkAPI) GetLaunchURLs() []string { return URLsFromArguments(os.Args[1:]) }

// URLsFromArguments selects custom-scheme URLs, excluding web links, CLI flags,
// and ordinary filesystem paths (including Windows drive paths).
func URLsFromArguments(arguments []string) []string {
	var urls []string
	for _, argument := range arguments {
		parsed, err := url.Parse(argument)
		if err != nil || parsed.Scheme == "" || strings.HasPrefix(argument, "-") {
			continue
		}
		scheme := strings.ToLower(parsed.Scheme)
		if scheme == "http" || scheme == "https" || (len(scheme) == 1 && len(argument) > 2 && (argument[2] == '/' || argument[2] == '\\')) {
			continue
		}
		urls = append(urls, parsed.String())
	}
	return urls
}

type secureStorageAPI struct{}

// SecureStorage keeps secrets in the operating system's credential store.
var SecureStorage secureStorageAPI

func (secureStorageAPI) IsSupported() (bool, error) {
	return callJSON[bool]("is-secure-storage-supported", nil)
}

func (secureStorageAPI) Set(service, account string, value []byte, done func(bool, error)) {
	// []byte is encoded as base64 by encoding/json. Empty bytes remain an explicit
	// empty secret rather than the absent value used by Get.
	if value == nil {
		value = []byte{}
	}
	invokeJSON("set-secure-storage", struct {
		Service string `json:"service"`
		Account string `json:"account"`
		Value   []byte `json:"value"`
	}{service, account, value}, done)
}

func (secureStorageAPI) SetText(service, account, value string, done func(bool, error)) {
	SecureStorage.Set(service, account, []byte(value), done)
}

// Get returns nil when no credential exists; an empty stored value is non-nil.
func (secureStorageAPI) Get(service, account string, done func([]byte, error)) {
	invokeJSON("get-secure-storage", map[string]string{"service": service, "account": account}, done)
}

func (secureStorageAPI) GetText(service, account string, done func(*string, error)) {
	SecureStorage.Get(service, account, func(value []byte, err error) {
		if done == nil {
			return
		}
		if value == nil {
			done(nil, err)
			return
		}
		text := string(value)
		done(&text, err)
	})
}

func (secureStorageAPI) Delete(service, account string, done func(bool, error)) {
	invokeJSON("delete-secure-storage", map[string]string{"service": service, "account": account}, done)
}

// CPUSampler reports CPU use since the previous sample from this handle.
// Use it on the application goroutine and release it when no longer needed.
type CPUSampler struct{ id uint32 }

func NewCPUSampler() (*CPUSampler, error) {
	id, err := callJSON[uint32]("cpu-sampler-create", nil)
	if err != nil {
		return nil, err
	}
	return &CPUSampler{id: id}, nil
}

func (sampler *CPUSampler) Sample() (CPUUsage, error) {
	if sampler == nil || sampler.id == 0 {
		return CPUUsage{}, fmt.Errorf("CPU sampler has been released")
	}
	return callJSON[CPUUsage]("cpu-sampler-sample", map[string]uint32{"id": sampler.id})
}

func (sampler *CPUSampler) Release() error {
	if sampler == nil || sampler.id == 0 {
		return nil
	}
	_, err := callJSON[bool]("cpu-sampler-release", map[string]uint32{"id": sampler.id})
	if err == nil {
		sampler.id = 0
	}
	return err
}

type CPUUsage struct {
	Percent         *float64 `json:"percent,omitempty"`
	IntervalSeconds float64  `json:"intervalSeconds"`
	CPUSeconds      float64  `json:"cpuSeconds"`
	TotalCPUSeconds float64  `json:"totalCpuSeconds"`
}

type ProcessMetrics struct {
	CPUUserSeconds   float64 `json:"cpuUserSeconds"`
	CPUSystemSeconds float64 `json:"cpuSystemSeconds"`
	ResidentBytes    uint64  `json:"residentBytes"`
	FootprintBytes   *uint64 `json:"footprintBytes,omitempty"`
	VirtualBytes     uint64  `json:"virtualBytes"`
	ThreadCount      *uint64 `json:"threadCount,omitempty"`
	UptimeSeconds    float64 `json:"uptimeSeconds"`
}

type SystemMemory struct {
	TotalBytes     uint64 `json:"totalBytes"`
	AvailableBytes uint64 `json:"availableBytes"`
	FreeBytes      uint64 `json:"freeBytes"`
	UsedBytes      uint64 `json:"usedBytes"`
}

type metricsAPI struct{}

var Metrics metricsAPI

func (metricsAPI) GetProcessMetrics(done func(ProcessMetrics, error)) {
	invokeJSON("get-process-metrics", struct{}{}, done)
}

func (metricsAPI) GetSystemMemory(done func(SystemMemory, error)) {
	invokeJSON("get-system-memory", struct{}{}, done)
}

// GetFrameMetrics reads the latest completed frame without requesting another frame.
// It returns nil before the window completes its first frame.
func (metricsAPI) GetFrameMetrics(window *Window, done func(*FrameMetrics, error)) {
	if done == nil {
		done = func(*FrameMetrics, error) {}
	}
	if window == nil {
		done(nil, fmt.Errorf("frame metrics require a window"))
		return
	}
	commandJSON(map[string]any{"method": "get-window-frame-metrics", "window": window.NativeID}, func(metrics FrameMetrics, err error) {
		if err != nil || metrics.FrameNumber == 0 {
			done(nil, err)
			return
		}
		done(&metrics, nil)
	})
}
