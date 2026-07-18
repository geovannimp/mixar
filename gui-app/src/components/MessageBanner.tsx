interface MessageBannerProps {
  message: string;
  variant: "error" | "success";
}

const variantClasses: Record<MessageBannerProps["variant"], string> = {
  error: "border-red-500/35 bg-red-500/15 text-red-200",
  success: "border-emerald-500/35 bg-emerald-500/10 text-emerald-200",
};

export function MessageBanner({ message, variant }: MessageBannerProps) {
  return (
    <div className={`rounded-xl border px-4 py-3 text-sm ${variantClasses[variant]}`}>
      {message}
    </div>
  );
}
