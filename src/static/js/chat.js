// Chat interface for board management via LLM
(function() {
    'use strict';

    const chatContainer = document.getElementById('chat-container');
    if (!chatContainer) return;

    const chatMessages = document.getElementById('chat-messages');
    const chatInput = document.getElementById('chat-input');
    const chatForm = document.getElementById('chat-form');
    const chatToggle = document.getElementById('chat-toggle');
    const boardId = chatContainer.dataset.boardId;

    let isExpanded = false;

    // Load chat history on page load
    loadHistory();

    // Toggle chat messages panel
    chatToggle.addEventListener('click', function() {
        isExpanded = !isExpanded;
        chatMessages.classList.toggle('expanded', isExpanded);
        chatToggle.innerHTML = isExpanded ?
            '<i class="bi bi-chevron-down"></i>' :
            '<i class="bi bi-chevron-up"></i>';
    });

    // Handle form submission
    chatForm.addEventListener('submit', async function(e) {
        e.preventDefault();

        const message = chatInput.value.trim();
        if (!message) return;

        // Show user message
        addMessage(message, 'user');
        chatInput.value = '';

        // Expand messages if not already
        if (!isExpanded) {
            isExpanded = true;
            chatMessages.classList.add('expanded');
            chatToggle.innerHTML = '<i class="bi bi-chevron-down"></i>';
        }

        // Show loading
        const loadingEl = addLoading();

        try {
            const response = await fetch(`/api/boards/${boardId}/chat`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ message: message }),
            });

            loadingEl.remove();

            if (!response.ok) {
                const error = await response.json().catch(() => ({ message: 'Request failed' }));
                addMessage(error.message || 'An error occurred', 'error');
                return;
            }

            const data = await response.json();
            addMessage(data.response, 'assistant', data.actions_taken);

            // If actions were taken that modified the board, refresh after a delay
            if (data.actions_taken && data.actions_taken.some(a => a.success)) {
                setTimeout(() => {
                    window.location.reload();
                }, 1500);
            }
        } catch (error) {
            loadingEl.remove();
            addMessage('Failed to connect to chat service. Is Ollama running?', 'error');
        }
    });

    async function loadHistory() {
        try {
            const response = await fetch(`/api/boards/${boardId}/chat/history`);
            if (!response.ok) return;

            const messages = await response.json();
            if (messages.length === 0) return;

            // Render history messages
            for (const msg of messages) {
                addMessage(msg.message, 'user', null, true);
                addMessage(msg.response, 'assistant', msg.actions_taken, true);
            }
        } catch (error) {
            console.error('Failed to load chat history:', error);
        }
    }

    function addMessage(content, type, actions, isHistory) {
        const msgEl = document.createElement('div');
        msgEl.className = `chat-message chat-message--${type}`;
        if (isHistory) {
            msgEl.classList.add('chat-message--history');
        }

        let html = `<div class="chat-message-content">${escapeHtml(content)}</div>`;

        if (actions && actions.length > 0) {
            html += '<div class="chat-message-actions">';
            for (const action of actions) {
                const cls = action.success ? 'action-success' : 'action-failed';
                const icon = action.success ? 'check-circle' : 'x-circle';
                html += `<div class="${cls}"><i class="bi bi-${icon}"></i> ${escapeHtml(action.description)}</div>`;
            }
            html += '</div>';
        }

        msgEl.innerHTML = html;
        chatMessages.appendChild(msgEl);
        chatMessages.scrollTop = chatMessages.scrollHeight;
        return msgEl;
    }

    function addLoading() {
        const loadingEl = document.createElement('div');
        loadingEl.className = 'chat-loading';
        loadingEl.innerHTML = '<div class="chat-loading-spinner"></div> Thinking...';
        chatMessages.appendChild(loadingEl);
        chatMessages.scrollTop = chatMessages.scrollHeight;
        return loadingEl;
    }

    function escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }
})();
