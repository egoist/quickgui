package main

import (
	"context"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

func TestConversationsKeepDraftsAndDoNotDuplicateEmptyChats(t *testing.T) {
	t.Setenv("DEEPSEEK_API_KEY", "")
	controller := newChatController(ConversationHistory{}, nil)
	first := controller.current().ID
	controller.newConversation()
	if len(controller.state.Peek().Conversations) != 1 {
		t.Fatal("empty chat duplicated")
	}
	controller.setDraft("unsent message")
	before := controller.state.Peek()
	controller.newConversation()
	second := controller.current().ID
	controller.setDraft("second draft")
	controller.selectConversation(first)
	if controller.current().Draft != "unsent message" || before.Conversations[0].Draft != "unsent message" {
		t.Fatal("draft or snapshot was mutated")
	}
	controller.send()
	if !controller.settingsOpen.Peek() || controller.current().Draft != "unsent message" || controller.busy.Peek() {
		t.Fatal("missing credential discarded draft or sent a request")
	}
	controller.busy.Write(true)
	controller.selectConversation(second)
	if controller.current().ID != first {
		t.Fatal("changed conversations during a response")
	}
}

func TestCompletionContextSkipsWelcomeAndFailedMessagesWithinBudget(t *testing.T) {
	messages := []ChatMessage{
		{Role: "assistant", Content: "welcome"},
		{Role: "user", Content: strings.Repeat("x", maxContextCharacters)},
		{Role: "assistant", Content: "failed", Failed: true},
		{Role: "assistant", Content: "in progress", Streaming: true},
		{Role: "user", Content: "latest prompt"},
	}
	context := completionContext(messages)
	if len(context) != 1 || context[0].Content != "latest prompt" {
		t.Fatal(context)
	}
}

func runCallback(t *testing.T, queue <-chan func()) {
	t.Helper()
	select {
	case callback := <-queue:
		callback()
	case <-time.After(2 * time.Second):
		t.Fatal("completion callback was not delivered")
	}
}

func TestStopAndQuitPreserveUnflushedResponseAndLatestDraft(t *testing.T) {
	t.Setenv("DEEPSEEK_API_KEY", "test-key")
	path := filepath.Join(t.TempDir(), "history.json")
	controller := newChatController(ConversationHistory{}, &historyWriter{path: path})
	queue := make(chan func(), 8)
	controller.dispatch = func(callback func()) { queue <- callback }
	started := make(chan struct{})
	controller.complete = func(ctx context.Context, key string, messages []Message, update func(string) error) (string, error) {
		if key != "test-key" || len(messages) != 1 || messages[0].Content != "First prompt" {
			panic("invalid completion request")
		}
		_ = update("published prefix")
		close(started)
		<-ctx.Done()
		return "published prefix and final tokens", ctx.Err()
	}
	controller.setDraft("First prompt")
	controller.send()
	select {
	case <-started:
	case <-time.After(2 * time.Second):
		t.Fatal("completion not started")
	}
	runCallback(t, queue)
	var flushed bool
	controller.shutdown(func(err error) {
		if err != nil {
			t.Error(err)
		}
		flushed = true
	})
	if flushed {
		t.Fatal("quit finished before response was finalized")
	}
	runCallback(t, queue)
	runCallback(t, queue)
	if !flushed || controller.busy.Peek() {
		t.Fatal("quit did not finish")
	}
	history, err := loadHistory(path, "")
	if err != nil {
		t.Fatal(err)
	}
	conversation := history.Conversations[0]
	last := conversation.Messages[len(conversation.Messages)-1]
	if last.Content != "published prefix and final tokens" || last.Streaming || conversation.Title != "First prompt" || conversation.Draft != "" {
		t.Fatal(conversation)
	}
}

func TestMessageCardsStayKeyedWhileOnlyResponseChanges(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		controller := newChatController(ConversationHistory{}, nil)
		id := controller.current().ID
		controller.edit(id, func(conversation *Conversation) {
			conversation.Messages = append(conversation.Messages, ChatMessage{ID: 2, Role: "user", Content: "hello"}, ChatMessage{ID: 3, Role: "assistant", Content: "first", Streaming: true})
		})
		roots := native.CollectChildren(controller.view)
		var markdown []*native.Node
		var list *native.Node
		var walk func(*native.Node)
		walk = func(node *native.Node) {
			if node.Tag == protocol.TagExtension {
				markdown = append(markdown, node)
			}
			if node.Tag == protocol.TagVirtualList {
				list = node
			}
			for _, child := range node.Children {
				walk(child)
			}
		}
		for _, root := range roots {
			walk(root)
		}
		if len(markdown) != 3 || list == nil {
			t.Fatal("transcript is not a list of separate Markdown messages")
		}
		original := append([]*native.Node(nil), markdown...)
		controller.updateResponse(id, 3, "second", true, false)
		markdown = nil
		for _, root := range roots {
			walk(root)
		}
		for index, node := range markdown {
			if node != original[index] {
				t.Fatal("stream update remounted a message")
			}
		}
		return struct{}{}
	})
}
