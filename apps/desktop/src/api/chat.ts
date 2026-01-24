// Chat API: stream SSE responses from local backend (127.0.0.1:8000)

export interface Message {
    role: 'system' | 'user' | 'assistant';
    content: string;
}

export interface ChatRequest {
    model: string;
    messages: Message[];
    max_tokens?: number;
    temperature?: number;
    stream?: boolean;
}

// Chat persistence: API response types for conversation management
export interface ConversationSummary {
    id: string;
    title: string;
    created_at: string;
    last_accessed: string;
    message_count: number;
}

export interface ConversationsResponse {
    conversations: ConversationSummary[];
}

export interface ConversationDetailResponse {
    id: string;
    title: string;
    messages: Message[];
}

// Backend base URL (Axum/Tauri API)
const API_Base = 'http://localhost:8000';

// Stream assistant tokens via Server-Sent Events (delta chunks)
export async function* streamChat(messages: Message[], sessionId?: string): AsyncGenerator<string, void, unknown> {
    // Enforce session ID requirement for persistence - prevents orphaned conversations
    if (!sessionId) {
        throw new Error('Session ID is required for chat. This is a bug - ChatWindow should have generated one.');
    }

    const response = await fetch(`${API_Base}/generate/stream`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({
            model: 'local-llm',
            messages,
            session_id: sessionId,
            max_tokens: 2000,
            stream: true,
            temperature: 0.7,
        } as ChatRequest),
    });

    if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
    }

    if (!response.body) {
        throw new Error('Response body is null');
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder('utf-8');
    let buffer = '';

    while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');

        // Keep the last incomplete line in the buffer
        buffer = lines.pop() || '';

        for (const line of lines) {
            const trimmed = line.trim();
            if (!trimmed || trimmed === '[DONE]') continue;

            if (trimmed.startsWith('data: ')) {
                try {
                    const jsonStr = trimmed.slice(6);
                    if (jsonStr === '[DONE]') continue;

                    const data = JSON.parse(jsonStr);
                    const content = data.choices?.[0]?.delta?.content;
                    if (content) {
                        yield content;
                    }
                } catch (e) {
                    console.error('Error parsing SSE line:', e);
                }
            }
        }
    }
}

// Chat persistence: Fetch all saved conversations for sidebar display
export async function fetchConversations(): Promise<ConversationSummary[]> {
    try {
        const response = await fetch(`${API_Base}/conversations`);
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data: ConversationsResponse = await response.json();
        return data.conversations;
    } catch (error) {
        console.error('Failed to fetch conversations:', error);
        return [];
    }
}

// Chat persistence: Load full conversation history from database when user clicks a chat
export async function fetchConversation(id: string): Promise<ConversationDetailResponse | null> {
    try {
        const response = await fetch(`${API_Base}/conversations/${id}`);
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return await response.json();
    } catch (error) {
        console.error('Failed to fetch conversation:', error);
        return null;
    }
}

// Chat persistence: Save auto-generated title to database after first message
export async function updateConversationTitle(id: string, title: string): Promise<boolean> {
    try {
        const response = await fetch(`${API_Base}/conversations/${id}/title`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ title }),
        });
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return true;
    } catch (error) {
        console.error('Failed to update conversation title:', error);
        return false;
    }
}

// Chat persistence: Delete a conversation permanently from the database
// Returns boolean to indicate success; caller handles user-facing error messages
export async function deleteConversation(id: string): Promise<boolean> {
    try {
        const response = await fetch(`${API_Base}/conversations/${id}`, {
            method: 'DELETE',
            headers: {
                'Content-Type': 'application/json',
            },
        });
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return true;  // Database deletion successful
    } catch (error) {
        console.error('Failed to delete conversation:', error);
        return false;  // Network or backend error
    }
}