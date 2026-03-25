import { useState, useEffect, useCallback } from "react";
import { adminListAirports } from "~/server/admin";

export function useAdminAuth() {
  const [authenticated, setAuthenticated] = useState<boolean | null>(null);

  const verify = useCallback(() => {
    const password = localStorage.getItem("admin_password");
    if (!password) {
      setAuthenticated(false);
      return;
    }
    adminListAirports({ data: password })
      .then(() => setAuthenticated(true))
      .catch(() => {
        localStorage.removeItem("admin_password");
        setAuthenticated(false);
      });
  }, []);

  useEffect(() => {
    verify();
  }, [verify]);

  return { authenticated, setAuthenticated };
}
