import React from "react";
import "./App.css";
import Linkify from "react-linkify";

interface ChatMessage {
  user: string;
  message: string;
}

function App() {
  const [history, setHistory] = React.useState<ChatMessage[]>([]);
  const [users, setUsers] = React.useState<string[]>([]);
  const ref = React.useRef<WebSocket>();
  React.useEffect(() => {
    if (!ref.current)
      ref.current = new WebSocket(
        `${window.location.protocol === "https:" ? "wss" : "ws"}://${window.location.host
        }/`
      );
  }, []);

  React.useEffect(() => {
    if (ref.current) {
      ref.current.onmessage = message => {
        const data = JSON.parse(message.data);
        if (data.type === "userlist") {
          setUsers(data.users);
        } else if (data.type === "message") {
          setHistory([...history, { user: data.user, message: data.message }]);
        }
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
    <div className="App container">
      <div className="row">
        <div className="col">
          <ul className="list-group list-group-flush">
            {history.map((message, i) =>
              message.user === "system" ? (
                <li
                  key={i}
                  className="list-group-item list-group-item-light font-italic"
                >
                  {message.message}
                </li>
              ) : (
                  <li key={i} className="list-group-item">
                    <strong>{message.user}</strong>{" "}
                    <Linkify>{message.message}</Linkify>
                  </li>
                )
            )}
          </ul>
          <input
            type="text"
            value={value}
            autoFocus
            onChange={onChange}
            onKeyPress={onKeyPress}
          />
        </div>
        <div className="col d-none d-sm-block">
          <ol className="list-group">
            <li className="list-group-item list-group-item-primary">Users</li>
            {users.map(user => (
              <li className="list-group-item" key={user}>
                {user}
              </li>
            ))}
          </ol>
        </div>
      </div>
    </div>
  );
}

export default App;
