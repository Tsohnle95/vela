import { useMemo, useState, type ReactNode } from "react";
import type { ModelOption, WorkspaceIdentity } from "@shared/types";
import { IconArrowLeft, IconCheck, IconChevronDown, IconChevronRight, IconEye, IconEyeClosed, IconGear, IconStarFilled } from "./icons";

function useModelGroups(models: ModelOption[]): [string, ModelOption[]][] {
  return useMemo(() => {
    const map = new Map<string, ModelOption[]>();
    for (const m of models) {
      const arr = map.get(m.providerID) ?? [];
      arr.push(m);
      map.set(m.providerID, arr);
    }
    return [...map.entries()]
      .sort((a, b) => a[0].localeCompare(b[0]))
      .map(([provider, list]) => [
        provider,
        list.sort((a, b) => a.name.localeCompare(b.name))
      ]) as [string, ModelOption[]][];
  }, [models]);
}
function modelKey(model: ModelOption): string {
  return `${model.providerID}::${model.id}`;
}

function readModelKeys(storageKey: string): Set<string> {
  try {
    const raw = window.localStorage.getItem(storageKey);
    const parsed = raw ? (JSON.parse(raw) as unknown) : [];
    return new Set(Array.isArray(parsed) ? parsed.filter((k): k is string => typeof k === "string") : []);
  } catch {
    return new Set();
  }
}

function writeModelKeys(storageKey: string, keys: Set<string>): void {
  window.localStorage.setItem(storageKey, JSON.stringify([...keys]));
}

export function formatVariant(variant: string | undefined): string {
  if (!variant) return "Auto";
  return variant
    .replace(/[-_]+/g, " ")
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

export function useComposerModels(models: ModelOption[], currentModel: ModelOption | null,
  switchModel: (id: string, providerID: string, variant?: string, workspace?: WorkspaceIdentity) => Promise<void>,
  workspace: WorkspaceIdentity | null, closeMenu: () => void) {
  const [favorites, setFavorites] = useState<Set<string>>(() => readModelKeys("favoriteModels"));
  const [hiddenModels, setHiddenModels] = useState<Set<string>>(() => readModelKeys("hiddenModels"));
  const [hiddenProviders, setHiddenProviders] = useState<Set<string>>(() => readModelKeys("hiddenProviders"));
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const [modelView, setModelView] = useState<"list" | "settings" | "strength">("list");
  const [modelSearch, setModelSearch] = useState("");
  const visibleModels = useMemo(
    () =>
      models.filter(
        (model) =>
          (!hiddenProviders.has(model.providerID) && !hiddenModels.has(modelKey(model))) ||
          (currentModel?.id === model.id && currentModel?.providerID === model.providerID)
      ),
    [models, hiddenModels, hiddenProviders, currentModel]
  );
  const filteredModels = useMemo(() => {
    const query = modelSearch.trim().toLowerCase();
    if (!query) return visibleModels;
    return visibleModels.filter((model) =>
      `${model.name} ${model.id} ${model.providerID}`.toLowerCase().includes(query)
    );
  }, [modelSearch, visibleModels]);
  const filteredGroups = useModelGroups(filteredModels);
  const settingsModels = useMemo(
    () => models.filter((model) => {
      const query = modelSearch.trim().toLowerCase();
      return !query || `${model.name} ${model.id} ${model.providerID}`.toLowerCase().includes(query);
    }),
    [modelSearch, models]
  );
  const settingsGroups = useModelGroups(settingsModels);
  const favoriteList = useMemo(() => {
    const query = modelSearch.trim().toLowerCase();
    return models
      .filter((model) => favorites.has(modelKey(model)))
      .filter(
        (model) =>
          !query || `${model.name} ${model.id} ${model.providerID}`.toLowerCase().includes(query)
      )
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [favorites, models, modelSearch]);
  const chooseModel = (model: ModelOption): void => {
    void switchModel(model.id, model.providerID, currentModel?.id === model.id ? currentModel.variant : undefined, workspace ?? undefined);
    closeMenu();
  };

  const chooseVariant = (variant?: string): void => {
    if (!currentModel) return;
    void switchModel(currentModel.id, currentModel.providerID, variant, workspace ?? undefined);
    closeMenu();
  };

  const cycleFavorite = (): void => {
    if (favoriteList.length === 0) return;
    const index = favoriteList.findIndex(
      (model) => model.id === currentModel?.id && model.providerID === currentModel?.providerID
    );
    chooseModel(favoriteList[(index + 1) % favoriteList.length]);
  };

  const cycleStrength = (): void => {
    const variants = currentModel?.variants;
    if (!currentModel || !variants || variants.length === 0) return;
    const options: (string | undefined)[] = [undefined, ...variants];
    const index = options.findIndex((option) => option === currentModel.variant);
    void switchModel(currentModel.id, currentModel.providerID, options[(index + 1) % options.length], workspace ?? undefined);
  };

  const toggleKey = (set: React.Dispatch<React.SetStateAction<Set<string>>>, key: string, storageKey?: string): void => {
    set((previous) => {
      const next = new Set(previous);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      if (storageKey) writeModelKeys(storageKey, next);
      return next;
    });
  };
  const toggleFavorite = (model: ModelOption): void => toggleKey(setFavorites, modelKey(model), "favoriteModels");
  const toggleModelVisible = (model: ModelOption): void => toggleKey(setHiddenModels, modelKey(model), "hiddenModels");
  const toggleProviderVisible = (provider: string): void => toggleKey(setHiddenProviders, provider, "hiddenProviders");
  const toggleCollapsed = (provider: string): void => toggleKey(setCollapsed, provider);
  const reloadPreferences = (): void => {
    setFavorites(readModelKeys("favoriteModels"));
    setHiddenModels(readModelKeys("hiddenModels"));
    setHiddenProviders(readModelKeys("hiddenProviders"));
  };
  return { modelView, setModelView, modelSearch, setModelSearch, filteredGroups, settingsGroups,
    favoriteList, favorites, hiddenModels, hiddenProviders, collapsed, chooseModel, chooseVariant,
    cycleFavorite, cycleStrength, toggleFavorite, toggleModelVisible, toggleProviderVisible,
    toggleCollapsed, reloadPreferences };
}

function ModelOptionRow({ model, currentModel, favorite, chooseModel, toggleFavorite }: {
  model: ModelOption;
  currentModel: ModelOption | null;
  favorite: boolean;
  chooseModel: (model: ModelOption) => void;
  toggleFavorite: (model: ModelOption) => void;
}): ReactNode {
  const selected = currentModel?.id === model.id && currentModel?.providerID === model.providerID;
  return (
    <button className={`composer-menu-item ${selected ? "selected" : ""}`} onClick={() => chooseModel(model)}>
      <span className="composer-menu-check">{selected ? <IconCheck /> : ""}</span>
      {model.name}
      <span className={`composer-menu-star ${favorite ? "on" : ""}`}
        title={favorite ? "Remove from favorites" : "Add to favorites"}
        onClick={(event) => { event.stopPropagation(); toggleFavorite(model); }}>
        {favorite ? "★" : "☆"}
      </span>
    </button>
  );
}
export function ComposerModelMenu({ state, currentModel, variantLabel }: {
  state: ReturnType<typeof useComposerModels>;
  currentModel: ModelOption | null;
  variantLabel: string;
}): ReactNode {
  const { modelView, setModelView, filteredGroups,
    favoriteList, favorites, collapsed, chooseModel, toggleFavorite, toggleCollapsed } = state;
  return (
            <>
              <div className="composer-menu-header">
                <span className="composer-menu-title">
                  {modelView === "settings" ? "Model settings" : modelView === "strength" ? "Response strength" : "Model selection"}
                </span>
                {modelView === "settings" || modelView === "strength" ? (
                  <button
                    className="composer-menu-tool"
                    title="Back to model list"
                    onClick={() => setModelView("list")}
                  >
                    <IconArrowLeft />
                  </button>
                ) : (
                  <button
                    className="composer-menu-tool"
                    title="Choose which models appear here"
                    onClick={() => setModelView("settings")}
                  >
                    <IconGear />
                  </button>
                )}
              </div>
              {modelView === "settings" ? (
                <ModelSettings state={state} />
              ) : modelView === "strength" ? (
                <ModelStrength currentModel={currentModel} chooseVariant={state.chooseVariant} />
              ) : (
                <>
                  <ModelSearch state={state} />
                  {currentModel?.variants && currentModel.variants.length > 0 && (
                    <button
                      className="composer-menu-item"
                      title="Change response strength"
                      onClick={() => setModelView("strength")}
                    >
                      <span className="composer-menu-check" />
                      Response strength
                      <span className="composer-menu-value">
                        {variantLabel}
                        <IconChevronRight />
                      </span>
                    </button>
                  )}
                  {favoriteList.length > 0 && (
                    <div className="composer-menu-group composer-menu-favorites">
                      <button
                        className="composer-menu-head"
                        title={collapsed.has("favorites") ? "Expand Favorites" : "Collapse Favorites"}
                        onClick={() => toggleCollapsed("favorites")}
                      >
                        {collapsed.has("favorites") ? <IconChevronRight /> : <IconChevronDown />}
                        <IconStarFilled />
                        Favorites
                      </button>
                      {!collapsed.has("favorites") && favoriteList.map((model) => (
                        <ModelOptionRow key={`fav::${model.id}::${model.providerID}`} model={model} currentModel={currentModel}
                          favorite={true} chooseModel={chooseModel} toggleFavorite={toggleFavorite} />
                      ))}
                    </div>
                  )}
                  {filteredGroups.map(([provider, list]) => (
                    <div key={provider} className="composer-menu-group">
                      <button
                        className="composer-menu-head"
                        title={collapsed.has(provider) ? `Expand ${provider}` : `Collapse ${provider}`}
                        onClick={() => toggleCollapsed(provider)}
                      >
                        {collapsed.has(provider) ? <IconChevronRight /> : <IconChevronDown />}
                        {provider}
                      </button>
                      {!collapsed.has(provider) && list.map((model) => (
                        <ModelOptionRow key={`${model.id}::${model.providerID}`} model={model} currentModel={currentModel}
                          favorite={favorites.has(modelKey(model))} chooseModel={chooseModel} toggleFavorite={toggleFavorite} />
                      ))}
                    </div>
                  ))}
                  {favoriteList.length === 0 && filteredGroups.length === 0 && (
                    <div className="composer-menu-empty">No models found.</div>
                  )}
                </>
              )}
            </>
  );
}

function ModelSettings({ state }: { state: ReturnType<typeof useComposerModels> }): ReactNode {
  const { settingsGroups, hiddenModels, hiddenProviders, collapsed,
    toggleCollapsed, toggleProviderVisible, toggleModelVisible } = state;
  return (
                <div className="composer-menu-settings">
                  <ModelSearch state={state} />
                  {settingsGroups.map(([provider, list]) => {
                    const providerVisible = !hiddenProviders.has(provider);
                    return (
                    <div key={provider} className="composer-menu-group">
                      <div className="composer-menu-head composer-menu-provider-head">
                        <button
                          className="composer-menu-head-toggle"
                          title={collapsed.has(provider) ? `Expand ${provider}` : `Collapse ${provider}`}
                          onClick={() => toggleCollapsed(provider)}
                        >
                          {collapsed.has(provider) ? <IconChevronRight /> : <IconChevronDown />}
                          {provider}
                        </button>
                        <button
                          className="composer-menu-provider-visibility"
                          title={providerVisible ? `Hide ${provider}` : `Show ${provider}`}
                          aria-label={providerVisible ? `Hide ${provider}` : `Show ${provider}`}
                          onClick={() => toggleProviderVisible(provider)}
                        >
                          {providerVisible ? <IconEye /> : <IconEyeClosed />}
                        </button>
                      </div>
                      {!collapsed.has(provider) && list.map((model) => {
                        const visible = providerVisible && !hiddenModels.has(modelKey(model));
                        return (
                          <button
                            key={`${model.id}::${model.providerID}`}
                            className={`composer-menu-item ${visible ? "" : "dimmed"}`}
                            onClick={() => toggleModelVisible(model)}
                          >
                            <span className="composer-menu-check">
                              {visible ? <IconEye /> : <IconEyeClosed />}
                            </span>
                            {model.name}
                          </button>
                        );
                      })}
                    </div>
                    );
                  })}
                  {settingsGroups.length === 0 && <div className="composer-menu-empty">No models found.</div>}
                </div>
  );
}

function ModelStrength({ currentModel, chooseVariant }: {
  currentModel: ModelOption | null;
  chooseVariant: (variant?: string) => void;
}): ReactNode {
  return (
                <div className="composer-menu-group variant-menu">
                  <button className="composer-menu-item" onClick={() => chooseVariant()}>
                    <span className="composer-menu-check">{!currentModel?.variant ? <IconCheck /> : ""}</span>
                    Auto
                  </button>
                  {currentModel?.variants?.map((variant) => (
                    <button
                      key={variant}
                      className={`composer-menu-item ${currentModel.variant === variant ? "selected" : ""}`}
                      onClick={() => chooseVariant(variant)}
                    >
                      <span className="composer-menu-check">{currentModel.variant === variant ? <IconCheck /> : ""}</span>
                      {formatVariant(variant)}
                    </button>
                  ))}
                </div>
  );
}

function ModelSearch({ state }: { state: ReturnType<typeof useComposerModels> }): ReactNode {
  const { modelSearch, setModelSearch } = state;
  return (
                  <input
                    className="composer-model-search"
                    type="search"
                    spellCheck={false}
                    autoCorrect="off"
                    autoCapitalize="off"
                    value={modelSearch}
                    onChange={(event) => setModelSearch(event.target.value)}
                    placeholder="Search models"
                    aria-label="Search models"
                  />
  );
}
