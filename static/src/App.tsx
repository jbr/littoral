import React from "react";
import "./App.css";

function App() {
  const [history, setHistory] = React.useState<string[]>([]);
  const ref = React.useRef<WebSocket>();
  React.useEffect(() => {
    ref.current = new WebSocket(
      `${window.location.protocol === "https:" ? "wss" : "ws"}://${window.location.host
      }/`
    );
  }, []);

  React.useEffect(() => {
    if (ref.current) {
      ref.current.onmessage = message => {
        const newHistory = [...history, message.data];
        console.log({ history, newHistory });
        setHistory(newHistory);
      };
    }
  }, [history]);

  const [value, setValue] = React.useState<string>("");

  const onChange = React.useCallback(
    event => {
      setValue(event.target.value);
    },
    [setValue]
  );

  const onKeyPress = React.useCallback(
    event => {
      if (event.code === "Enter") {
        setValue("");
        ref.current?.send(event.target.value);
      }
    },
    [setValue]
  );

  return (
    <div className="App">
      <ul>
        {history.map(message => (
          <li key={message}>{message}</li>
        ))}
      </ul>
      <input
        type="text"
        value={value}
        onChange={onChange}
        onKeyPress={onKeyPress}
      />
    </div>
  );
}

export default App;
