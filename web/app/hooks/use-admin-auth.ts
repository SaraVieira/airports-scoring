import { useEffect } from "react";
import { useAuthStore } from "~/stores/admin";

export function useAdminAuth() {
  const { authenticated, verify, setPassword, logout } = useAuthStore();

  useEffect(() => {
    if (authenticated === null) {
      verify();
    }
  }, [authenticated, verify]);

  return { authenticated, setPassword, logout };
}
