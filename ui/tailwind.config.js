/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        ag: {
          bg: 'rgb(var(--ag-bg) / <alpha-value>)',
          card: 'rgb(var(--ag-card) / <alpha-value>)',
          surface: 'rgb(var(--ag-surface) / <alpha-value>)',
          border: 'rgb(var(--ag-border) / <alpha-value>)',
          text: 'rgb(var(--ag-text) / <alpha-value>)',
          muted: 'rgb(var(--ag-muted) / <alpha-value>)',
          primary: 'rgb(var(--ag-primary) / <alpha-value>)',
          success: 'rgb(var(--ag-success) / <alpha-value>)',
          danger: 'rgb(var(--ag-danger) / <alpha-value>)'
        }
      },
      boxShadow: {
        ag: '0 10px 30px rgba(15, 23, 42, 0.06)',
        soft: '0 4px 14px rgba(15, 23, 42, 0.08)'
      }
    }
  },
  plugins: []
}
