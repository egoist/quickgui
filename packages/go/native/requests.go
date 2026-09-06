package native

import (
	"encoding/json"
	"fmt"
	"strings"

	"github.com/egoist/quickgui/packages/go/host"
)

const readyRequest uint32 = 1

var nextRequest uint32 = 2
var appID uint32
var appReady bool

var pendingReplies = map[uint32]func(string, error){}

func setAppContext(id uint32, ready bool) {
	appID = id
	appReady = ready
}

func allocateRequest() uint32 {
	request := nextRequest
	nextRequest++
	if nextRequest >= 0xffff_fff0 {
		nextRequest = 2
	}
	return request
}

type dialogReply struct {
	complete func(value string, paths []string, err error)
}

var pendingDialogs = map[uint32]dialogReply{}

func settleReply(request uint32, value string, err error) {
	done, ok := pendingReplies[request]
	if !ok {
		return
	}
	delete(pendingReplies, request)
	done(value, err)
}

func settleDialog(request uint32, value string, paths []string, err error) {
	pending, ok := pendingDialogs[request]
	if !ok {
		return
	}
	delete(pendingDialogs, request)
	pending.complete(value, paths, err)
}

func rejectAllReplies(err error) {
	for request, done := range pendingReplies {
		delete(pendingReplies, request)
		done("", err)
	}
	for request, pending := range pendingDialogs {
		delete(pendingDialogs, request)
		pending.complete("", nil, err)
	}
}

func assertAppReady() {
	if appID == 0 || !appReady {
		panic("call native.Run before using the native QuickGUI application")
	}
}

// SendMutation queues one fire-and-forget system command.
func SendMutation(payload string) {
	assertAppReady()
	host.Current.Command(appID, 0, payload)
}

// SendCommand queues a system command and invokes done when the host replies.
func SendCommand(payload string, done func(json string, err error)) {
	if appID == 0 || !appReady {
		done("", fmt.Errorf("call native.Run before using the native QuickGUI application"))
		return
	}
	request := allocateRequest()
	pendingReplies[request] = done
	host.Current.Command(appID, request, payload)
}

func isNullJSON(text string) bool {
	return text == "" || text == "null"
}

// CallService answers one CPU-only host service synchronously and returns the JSON text of its value.
// The router uses this channel; it never waits on native main-thread execution.
func CallService(method, params string) (string, error) {
	return callService(method, params)
}

func callService(method, params string) (string, error) {
	replyText := host.Current.Call(method, params)
	var status struct {
		OK    bool   `json:"ok"`
		Error string `json:"error"`
	}
	if err := json.Unmarshal([]byte(replyText), &status); err != nil {
		return "", fmt.Errorf("the native service reply was malformed")
	}
	if !status.OK {
		if status.Error == "" {
			status.Error = "the native service failed"
		}
		return "", fmt.Errorf("%s", status.Error)
	}
	const prefix = `{"ok":true,"value":`
	if !strings.HasPrefix(replyText, prefix) || !strings.HasSuffix(replyText, "}") {
		return "", fmt.Errorf("the native service reply was malformed")
	}
	return replyText[len(prefix) : len(replyText)-1], nil
}
