/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        ag: {
          bg: "#FAFBFC",
          card: "#FFFFFF",
          border: "#E6EAF0",
          text: "#0F172A",
          muted: "#64748B",
          primary: "#2563EB",
          success: "#16A34A",
          danger: "#DC2626"
        }
      },
      boxShadow: {
        ag: "0 10px 30px rgba(15, 23, 42, 0.06)",
        soft: "0 4px 14px rgba(15, 23, 42, 0.08)"
      }
    }
  },
  plugins: []
}
