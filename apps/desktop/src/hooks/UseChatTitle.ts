// useChatTitle: Hook for generating chat titles from prompts via backend
import { useState } from 'react';

const API_BASE = 'http://localhost:8000';

export interface GenerateTitleRequest {
  prompt: string;
  max_tokens?: number;
}

export interface GenerateTitleResponse {
  title: string;
}

export const useChatTitle = () => {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const generateTitle = async (prompt: string): Promise<string | null> => {
    if (!prompt.trim()) {
      return null;
    }

    setLoading(true);
    setError(null);

    try {
      const response = await fetch(`${API_BASE}/generate/title`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          prompt: prompt.trim(),
          max_tokens: 20,
        } as GenerateTitleRequest),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const data: GenerateTitleResponse = await response.json();
      return data.title;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Unknown error';
      setError(errorMsg);
      console.error('Failed to generate title:', errorMsg);
      return null;
    } finally {
      setLoading(false);
    }
  };

  return { generateTitle, loading, error };
};