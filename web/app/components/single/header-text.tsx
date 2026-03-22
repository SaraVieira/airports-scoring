export const HeaderText = ({ children }: { children: React.ReactNode }) => {
  return (
    <h3 className="font-grotesk text-[13px] font-bold text-yellow-400 tracking-[2px] uppercase">
      {children}
    </h3>
  );
};
