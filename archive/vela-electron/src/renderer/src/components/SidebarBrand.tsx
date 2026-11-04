import observatoryWordmark from "../assets/observatory-wordmark.svg";

export function SidebarBrand(): React.ReactNode {
  return <span className="sidebar-brand"><span className="sidebar-brand-text">Vela</span><img className="observatory-wordmark" src={observatoryWordmark} alt="Vela" draggable={false} /></span>;
}
