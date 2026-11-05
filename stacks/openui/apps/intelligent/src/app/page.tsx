"use client";
import "@/lib/devtools";
import "@openuidev/react-ui/styles/index.css";

import {
  AgentInterface,
  fetchLLM,
  openAIConversationMessageFormat,
  openAIResponsesAdapter,
  useThemeModePreference,
  useThread,
} from "@openuidev/react-ui";
import type { ActionEvent } from "@openuidev/react-lang";
import { useEffect, useRef, useState } from "react";
import { ChartColumn, ChefHat, Dog, Drum, Hammer, MapPinned, Plane, UtensilsCrossed } from "lucide-react";
import { runLibraryAction } from "@/lib/actions";
import { withBase } from "@/lib/base-path";
import { localChatStorage, SaveMessages } from "@/lib/chat-storage";
import { webUrl } from "@/lib/components/style";
import { library } from "@/lib/library";
import { ModelPicker, chooseModel } from "@/components/model-picker";

// The chat opens @OpenUrl with a plain window.open of whatever URL the model wrote, and on
// openui.com this page shares its origin with the rest of the site. So only http(s) URLs open,
// in a new tab without an opener; a javascript: or data: URL does nothing.
if (typeof window !== "undefined") {
  const open = window.open.bind(window);
  window.open = (url?: string | URL) => {
    const safe = webUrl(String(url ?? ""));
    return safe ? open(safe, "_blank", "noopener,noreferrer") : null;
  };
}

// The examples OpenUI uses across its marketing: the homepage showcase (KALLAX, Kyoto),
// the README video (San Francisco) and the Intelligent UI blog posts (the rest).
const starters = [
  { displayText: "Build an IKEA KALLAX", prompt: "I just bought an IKEA KALLAX. Can you walk me through building it?", icon: <Hammer size={16} /> },
  { displayText: "Plan a day in San Francisco", prompt: "I'm in San Francisco for a day, plan a sightseeing route for me", icon: <MapPinned size={16} /> },
  { displayText: "Visit Kyoto in November", prompt: "Plan a slow four-day trip to Kyoto in November, with temples, gardens and a map", icon: <Plane size={16} /> },
  { displayText: "Plan a dinner menu", prompt: "Plan a dinner menu for friends, with a shopping list that scales with the number of guests", icon: <UtensilsCrossed size={16} /> },
  { displayText: "Cook a slow-roasted lamb", prompt: "Walk me through a slow-roasted lamb shoulder with potatoes and herbs", icon: <ChefHat size={16} /> },
  { displayText: "Design a dog vest", prompt: "Make a moodboard for a dog vest, with material options and colour choices", icon: <Dog size={16} /> },
  { displayText: "Build a sales dashboard", prompt: "Build an analytics dashboard over demo sales data, with a chart and a what-if scenario slider", icon: <ChartColumn size={16} /> },
  { displayText: "Jam in a drum lab", prompt: "Make a drum lab I can play: a 16-step sequencer with tempo, swing and a few performance pads", icon: <Drum size={16} /> },
];

const llm = fetchLLM({
  url: withBase("/api/chat"),
  streamAdapter: openAIResponsesAdapter(),
  messageFormat: openAIConversationMessageFormat,
});

// Threads and messages persist in localStorage (created once, on the client).
const storage = typeof window === "undefined" ? undefined : localChatStorage();

// The chat sends @ToAssistant and opens @OpenUrl itself; every other action,
// such as the library's @CopyToClipboard, reaches onAction.
function onAction(event: ActionEvent) {
  void runLibraryAction(event);
}

// The static GnX landing hands over an explicit operator question.
function PromptFromLink() {
  const sent = useRef(false);
  const processMessage = useThread((state) => state.processMessage);
  useEffect(() => {
    if (sent.current) return;
    sent.current = true;
    const url = new URL(window.location.href);
    const prompt = url.searchParams.get("prompt")?.trim().slice(0, 600);
    if (!prompt) return;
    url.searchParams.delete("prompt");
    window.history.replaceState(null, "", url.pathname + url.search + url.hash);
    processMessage({ role: "user", content: prompt });
  }, [processMessage]);
  return null;
}

function ModelComposer() {
  const processMessage = useThread((state) => state.processMessage);
  const isRunning = useThread((state) => state.isRunning);
  const cancelMessage = useThread((state) => state.cancelMessage);
  const isLoadingMessages = useThread((state) => state.isLoadingMessages);
  const [text, setText] = useState('');
  const [notice, setNotice] = useState('');
  const send = async () => {
    const message = text.trim();
    if (!message || isRunning || isLoadingMessages) return;
    const match = message.match(/^\/model\s+(\S+)$/);
    if (match) {
      try { await chooseModel(match[1]); setNotice(`Modelo activo: ${match[1]}`); setText(''); }
      catch (e) { setNotice(e instanceof Error ? e.message : 'No se pudo seleccionar'); }
      return;
    }
    setNotice(''); setText('');
    processMessage({ role: 'user', content: message });
  };
  return <div style={{padding:'8px 16px', width:'100%'}}>
    <ModelPicker />
    <form onSubmit={e => { e.preventDefault(); void send(); }} style={{display:'flex', gap:8}}>
      <textarea aria-label="Mensaje" value={text} onChange={e => setText(e.target.value)} onKeyDown={e => { if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) { e.preventDefault(); void send(); } }} placeholder="Escribe un mensaje o /model nombre" rows={2} style={{flex:1, padding:12, resize:'vertical'}} />
      {isRunning ? <button type="button" onClick={cancelMessage}>Detener</button> : <button type="submit" disabled={isLoadingMessages || !text.trim()}>Enviar</button>}
    </form>
    <span role="status">{notice}</span>
  </div>;
}

export default function Home() {
  // Follows the system theme until the footer toggle is first used, then
  // remembers the choice.
  const { setPreference, mode } = useThemeModePreference();

  // Mirror the resolved mode onto <html>, so the components' dark tokens also
  // apply while the theme follows the system.
  useEffect(() => {
    document.documentElement.setAttribute("data-openui-theme", mode);
  }, [mode]);

  return (
    <div className="chat-shell">
      <AgentInterface
        llm={llm}
        storage={storage}
        componentLibrary={library}
        onAction={onAction}
        agentName="OpenUI"
        logoUrl={withBase("/shiro-face.svg")}
        theme={{ mode }}
        starters={starters}
      >
        <AgentInterface.SidebarFooter>
          <AgentInterface.ThemeModeToggle mode={mode} onModeChange={setPreference} />
        </AgentInterface.SidebarFooter>
        <AgentInterface.ThreadHeader showChatTitle={false} />
        <AgentInterface.Composer><ModelComposer /></AgentInterface.Composer>
        <AgentInterface.Welcome
          image={<img src={withBase("/shiro-woodworker.svg")} alt="" className="welcome-mascot" />}
          title="What can I help with?"
          starterVariant="card"
          glowAnimation
        />
        <SaveMessages />
        <PromptFromLink />
      </AgentInterface>
    </div>
  );
}
