import { Button } from "~/components/ui/button";
import { Card, CardContent } from "../ui/card";
import { useState } from "react";
import { useAuthStore } from "~/stores/admin";

export function LoginForm({ onLogin }: { onLogin: (password: string) => void }) {
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError("");
    try {
      // Set password in store first, then verify via store
      useAuthStore.getState().setPassword(password);
      const valid = await useAuthStore.getState().verify();
      if (valid) {
        onLogin(password);
      } else {
        setError("Invalid password");
      }
    } catch {
      setError("Invalid password");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-background flex items-center justify-center">
      <Card className="w-full max-w-sm">
        <CardContent className="pt-6">
          <form onSubmit={handleSubmit} className="flex flex-col gap-4">
            <h1 className="font-grotesk text-lg font-bold">Admin Login</h1>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Password"
              className="w-full bg-muted border border-border text-foreground text-sm px-3 py-2 rounded-md focus:outline-none focus:ring-1 focus:ring-ring"
              autoFocus
            />
            {error && <p className="text-destructive text-xs">{error}</p>}
            <Button type="submit" disabled={loading} className="w-full">
              {loading ? "Verifying..." : "Login"}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
